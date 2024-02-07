use reqwest::header::USER_AGENT;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::path::Path;
use std::io::Read;

use crate::utilities::update_tauri_config_endpoint;


#[derive(Debug, Serialize, Deserialize)]
pub struct Asset {
    url: String,                  // This is the API URL, which includes the asset ID.
    browser_download_url: String, // This is the direct download URL for the asset.
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Release {
    name: String,
    pub upload_url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GistContent {
    pub version: String,
    pub notes: String,
    pub pub_date: String,
    pub platforms: HashMap<String, PlatformDetail>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlatformDetail {
    pub signature: String,
    pub url: String,
}

pub async fn get_latest_release(
    github_user_repo: &str,
    new_version: &str,
    release_notes: &str,
    github_pat: &str,
) -> Result<Release, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        github_user_repo
    );

    println!("\nChecking releases at: {}", url);

    let response = client
        .get(&url)
        .header("User-Agent", "reqwest")
        .bearer_auth(github_pat)
        .send()
        .await;

    match response {
        Ok(resp) => match resp.status() {
            StatusCode::OK => {
                let release = resp.json::<Release>().await?;
                println!("Evaluating Release versions...");

                if new_version == release.name {
                    println!(
                        "New version {} is equal to the latest Release name. Using this Release URL for upload...",
                        new_version
                    );
                    Ok(release)
                } else {
                    println!(
                        "New version {} is not equal to the latest release name {}. Creating new Release ...",
                        new_version, release.name
                    );
                    create_github_release(github_user_repo, new_version, release_notes, github_pat)
                        .await
                }
            }
            StatusCode::NOT_FOUND => {
                println!("No existing release found. Creating a new one...");
                create_github_release(github_user_repo, new_version, release_notes, github_pat)
                    .await
            }
            _ => Err(format!(
                "Error fetching the latest release: HTTP Status {}",
                resp.status()
            )
            .into()),
        },
        Err(_e) => {
            // For simplicity, directly attempt to create a new release if there's an error
            // You might want to handle different errors differently
            println!("Error fetching the latest release. Attempting to create a new one...");
            create_github_release(github_user_repo, new_version, release_notes, github_pat).await
        }
    }
}

pub async fn create_github_release(
    repo: &str,
    tag: &str,
    release_notes: &str,
    token: &str,
) -> Result<Release, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let url = format!("https://api.github.com/repos/{}/releases", repo);

    println!("Posting Release to url : \n{}", url);

    let response = client
        .post(url)
        .header(USER_AGENT, "javelin")
        .bearer_auth(token)
        .json(&json!({
            "tag_name": tag,
            "name": tag,
            "body": release_notes.to_string(),
            "draft": false,
            "prerelease": false,
        }))
        .send()
        .await?
        .error_for_status()?
        .json::<Release>()
        .await?;

    Ok(response)
}

pub async fn upload_release_asset(
    upload_url: &str,
    filename: &Path,
    token: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    // Ensure the URL is correctly constructed to upload the asset
    let url = upload_url.replace(
        "{?name,label}",
        &format!("?name={}", filename.file_name().unwrap().to_str().unwrap()),
    );

    let mut file = File::open(filename)?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;

    // Perform the POST request to upload the asset
    let response = client
        .post(url)
        .header("Content-Type", "application/octet-stream")
        .header("Authorization", format!("token {}", token))
        .body(contents)
        .send()
        .await?;

    // Check if the request was successful and parse the JSON response
    if response.status().is_success() {
        let asset: Asset = response.json().await?;
        println!("Asset uploaded: {}", asset.url);
        Ok(asset.url) // Return the URL that includes the asset ID
    } else {
        // Handle error response...
        Err(format!("Failed to upload asset. Status: {} : You may be trying to overwrite a current arch artifact. Try increase the version number?", response.status()).into())
    }
}

pub async fn create_and_upload_gist(
    github_repo: &str,
    github_username: &str,
    token: &str,
    gist_content: &GistContent,
    platform_key: &str,
    tauri_config_path: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    let gist_file_content = serde_json::to_string_pretty(&gist_content)?;

    let description = format!("{}-javelin-{}", github_repo, platform_key);
    let filename = format!("{}-javelin-{}-manifest.json", github_repo, platform_key);

    println!("Uploading Gist Filename : {}", filename);
    println!("Description : {}", description);

    // Construct the JSON payload for the Gist API
    let payload = json!({
        "description": description.to_string(),
        "public": false,
        "files": {
            filename.to_string(): {
                "content": gist_file_content
            }
        }
    });

    // Send the request to create the Gist
    let response = client
        .post("https://api.github.com/gists")
        .bearer_auth(token)
        .header(USER_AGENT, "javelin") // Replace with your app identifier
        .json(&payload)
        .send()
        .await?
        .error_for_status()?;

    if response.status().is_success() {
        let gist_response: serde_json::Value = response.json().await?;
        let gist_endpoint = gist_response["id"].as_str();

        println!("Gist ID : {:?}", gist_endpoint);

        if let Some(gist_id) = gist_response["id"].as_str() {
            let gist_updater_endpoint = format!(
                "https://gist.github.com/{}/{}/raw",
                github_username, gist_id
            );
            update_tauri_config_endpoint(tauri_config_path, &gist_updater_endpoint)?;
            Ok(gist_id.to_string())
        } else {
            Err("Gist created but no ID returned".into())
        }
    } else {
        // Add this inside your error handling block
        if let Err(e) = response.error_for_status_ref() {
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to get error body".into());
            println!("Error: {:?}, Body: {}", e, error_body);
            return Err(format!("Failed to create gist. Error: {}", error_body).into());
        }
        println!("Passed Gist error");
        let error_text = response.text().await?;

        Err(format!("Failed to create gist. Error: {}", error_text).into())
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn fetch_and_update_gist(
    github_repo: &str,
    token: &str,
    gist_id: &str,
    new_version: &str,
    new_notes: &str,
    new_pub_date: &str,
    platform_key: &str,
    new_platform_detail: PlatformDetail,
   
) -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();
    // Fetch the gist
    let gist_url = format!("https://api.github.com/gists/{}", gist_id);
    let response = client
        .get(&gist_url)
        .header("User-Agent", "javelin")
        .bearer_auth(token)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("Failed to fetch gist: Status code {}", response.status()).into());
    }

    let mut gist: HashMap<String, Value> = response.json().await?;

    let filename = format!("{}-javelin-{}-manifest.json", github_repo, platform_key);

    if let Some(file) = gist
        .get_mut("files")
        .and_then(|f| f.as_object_mut())
        .and_then(|files| files.get_mut(&filename))
    {
        if let Some(content) = file.get("content").and_then(|c| c.as_str()) {
            let mut existing_content: GistContent = serde_json::from_str(content)?;

            // Update the version, notes, and pub_date fields
            existing_content.version = new_version.to_string();
            existing_content.notes = new_notes.to_string();
            existing_content.pub_date = new_pub_date.to_string();

            // Update or add the platform detail
            existing_content
                .platforms
                .insert(platform_key.to_string(), new_platform_detail.clone());

            // Serialize the updated content
            let updated_content = serde_json::to_string_pretty(&existing_content)?;

            let update_payload = json!({
                "files": {
                    filename: {
                        "content": updated_content
                    }
                }
            });

            let update_response = client
                .patch(&gist_url)
                .header("User-Agent", "javelin")
                .bearer_auth(token)
                .json(&update_payload)
                .send()
                .await?;

            if !update_response.status().is_success() {
                return Err(format!(
                    "Failed to update gist: Status code {}",
                    update_response.status()
                )
                .into());
            }
        } else {
            return Err("file content not found".into());
        }
    } else {
        return Err("File not found in the gist".into());
    }

    Ok(())
}
