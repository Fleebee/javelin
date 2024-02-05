use chrono::Utc;
use reqwest::header;
use reqwest::header::USER_AGENT;
use reqwest::Client;
use reqwest::StatusCode;

use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{self, Read};
use std::process::Command;
use std::{env, fs, path::Path};

#[derive(Deserialize, Debug)]
struct Config {
    github_username: String,
    github_repo: String,
    github_pat: String,
    secret_key_location: String,
    secret_key_password: String,
    gist_id: String,
}

#[derive(Deserialize, Debug)]
struct TauriConfig {
    package: Package,
    tauri: Tauri,
}
#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct Package {
    productName: String,
    version: String,
}

#[derive(Deserialize, Debug)]
struct Tauri {
    updater: Updater,
}

#[derive(Deserialize, Debug)]
struct Updater {
    pubkey: String,
    endpoints: Vec<String>,
}

#[derive(Debug)]
enum UpdateType {
    Major,
    Minor, // Using Minor instead of Feature for conventional naming
    Patch,
    Current,
}
fn update_tauri_config_endpoint(
    config_path: &str,
    new_endpoint: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Read the current configuration file
    let config_contents = fs::read_to_string(config_path)?;
    let mut config: Value = serde_json::from_str(&config_contents)?;

    // Navigate to the updater.endpoints array and update it
    if let Some(updater) = config["tauri"]["updater"].as_object_mut() {
        updater["endpoints"] = serde_json::json!([new_endpoint]);
    } else {
        return Err("Failed to find updater configuration in Tauri config".into());
    }

    // Write the updated configuration back to the file
    fs::write(config_path, serde_json::to_string_pretty(&config)?)?;

    Ok(())
}

macro_rules! exit_with_error {
    ($config_path:expr, $current_version:expr) => {{
        println!("Error occurred in file: {}, line: {}", file!(), line!());
        let result = reset_version_in_config($config_path, $current_version);
        std::process::exit(1);
    }};
}



fn read_and_update_version<P: AsRef<Path>>(
    path: P,
    update_type: UpdateType,
) -> Result<String, Box<dyn std::error::Error>> {
    let file_content = fs::read_to_string(&path)?;
    let mut json: Value = serde_json::from_str(&file_content)?;

    // Extract the current version string and update it
    let new_version = if let Some(version_str) = json["package"]["version"].as_str() {
        let new_version = update_version(version_str, update_type)?;
        // Update the version in the JSON object
        json["package"]["version"] = Value::String(new_version.clone());
        new_version
    } else {
        return Err("Version not found in the specified file".into());
    };

    // Write the updated JSON back to the file
    fs::write(path, serde_json::to_string_pretty(&json)?)?;

    // Return the new version
    Ok(new_version)
}

fn update_version(current_version: &str, update_type: UpdateType) -> Result<String, &'static str> {
    let mut segments: Vec<u32> = current_version
        .split('.')
        .map(|s| s.parse::<u32>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "Failed to parse version segments")?;

    if segments.len() != 3 {
        return Err("Version string does not have three segments");
    }

    match update_type {
        UpdateType::Major => {
            segments[0] += 1; // Increment major
            segments[1] = 0; // Reset minor
            segments[2] = 0; // Reset patch
        }
        UpdateType::Minor => {
            segments[1] += 1; // Increment minor
            segments[2] = 0; // Reset patch
        }
        UpdateType::Patch => {
            segments[2] += 1; // Increment patch
        }
        UpdateType::Current => {
            segments[2] += 0; // Increment patch
        }
    }

    Ok(format!("{}.{}.{}", segments[0], segments[1], segments[2]))
}

fn reset_version_in_config(
    config_path: &str,
    reset_version: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Read the current configuration
    let config_contents = fs::read_to_string(config_path)?;
    let mut config: Value = serde_json::from_str(&config_contents)?;

    // Assuming the version is at the root of the JSON structure
    if let Some(obj) = config.as_object_mut() {
        obj.insert(
            "version".to_string(),
            Value::String(reset_version.to_string()),
        );
    } else {
        return Err("Expected a JSON object at the root of the configuration".into());
    }

    // Write the updated configuration back to the file
    fs::write(config_path, serde_json::to_vec_pretty(&config)?)?;

    Ok(())
}

fn update_entry_in_config(
    config_path: &str,
    key_path: &[&str],
    new_value: &str,
) -> Result<(), Box<dyn Error>> {
    // Read the current configuration
    let config_contents = fs::read_to_string(config_path)?;
    let mut config: Value = serde_json::from_str(&config_contents)?;

    // Navigate to the specified key
    let mut current = &mut config;
    for &key in key_path.iter().take(key_path.len() - 1) {
        current = current.get_mut(key).ok_or("Key path not found")?;
    }

    // Assuming the last element in `key_path` is the actual key to update
    if let Some(obj) = current.as_object_mut() {
        let key = key_path.last().ok_or("Key path is empty")?;
        obj.insert(key.to_string(), Value::String(new_value.to_string()));
    } else {
        return Err("Expected a JSON object at the specified path".into());
    }

    // Write the updated configuration back to the file
    fs::write(config_path, serde_json::to_string_pretty(&config)?)?;

    Ok(())
}

fn read_config<P: AsRef<Path>>(path: P) -> Result<Config, Box<dyn std::error::Error>> {
    let config_str = fs::read_to_string(path)?;
    let config: Config = serde_json::from_str(&config_str)?;
    Ok(config)
}

fn read_tauri_config<P: AsRef<Path>>(path: P) -> Result<TauriConfig, Box<dyn std::error::Error>> {
    let tauri_config_str = fs::read_to_string(path)?;
    let tauri_config: TauriConfig = serde_json::from_str(&tauri_config_str)?;
    Ok(tauri_config)
}


// Assuming the Release struct and create_github_release function are defined elsewhere

async fn get_latest_release(
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
                    create_github_release(github_user_repo, new_version, release_notes, github_pat).await
                }
            },
            StatusCode::NOT_FOUND => {
                println!("No existing release found. Creating a new one...");
                create_github_release(github_user_repo, new_version, release_notes, github_pat).await
            },
            _ => Err(format!("Error fetching the latest release: HTTP Status {}", resp.status()).into()),
        },
        Err(_e) => {
            // For simplicity, directly attempt to create a new release if there's an error
            // You might want to handle different errors differently
            println!("Error fetching the latest release. Attempting to create a new one...");
            create_github_release(github_user_repo, new_version, release_notes, github_pat).await
        },
    }
}




async fn create_github_release(
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
        .header(USER_AGENT, "tauri_javelin")
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

#[derive(Debug, Serialize, Deserialize)]
struct Asset {
    url: String,                  // This is the API URL, which includes the asset ID.
    browser_download_url: String, // This is the direct download URL for the asset.
}

async fn upload_release_asset(
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

#[derive(Debug, Serialize, Deserialize)]
struct Release {
    name: String,
    upload_url: String,
}

#[derive(Debug, Deserialize)]
struct ReleaseGet {
    id: i64,
    tag_name: String,
    name: String,
    body: Option<String>,
}

async fn get_releases(repo: &str, token: &str) -> Result<Vec<ReleaseGet>, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let url = format!("https://api.github.com/repos/{}/releases", repo);

    let response = client
        .get(&url)
        .bearer_auth(token)
        .header(header::USER_AGENT, "reqwest")
        .send()
        .await?
        .error_for_status()?;

    let releases = response.json::<Vec<ReleaseGet>>().await?;

    Ok(releases)
}

#[derive(Debug, Serialize, Deserialize)]
struct GistFile {
    filename: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Gist {
    files: HashMap<String, GistFile>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
struct GistContent {
    version: String,
    notes: String,
    pub_date: String,
    platforms: HashMap<String, PlatformDetail>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PlatformDetail {
    signature: String,
    url: String,
}

async fn create_and_upload_gist(
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
        .header(USER_AGENT, "tauri_javelin") // Replace with your app identifier
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

async fn update_gist(
    token: &str,
    gist_id: &str,
    gist_content: &GistContent,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let gist_file_content = serde_json::to_string_pretty(&gist_content)?;

    let payload = json!({
        "description": "Updated Tauri App Info",
        "files": {
            "update_info.json": {
                "content": gist_file_content
            }
        }
    });

    let url = format!("https://api.github.com/gists/{}", gist_id);

    let response = client
        .patch(&url)
        .bearer_auth(token)
        .header("User-Agent", "MyRustApp")
        .json(&payload)
        .send()
        .await?;

    if response.status().is_success() {
        println!("Gist updated successfully.");
    } else {
        let error_text = response.text().await?;
        return Err(format!("Failed to update gist: {}", error_text).into());
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    let config_path = "tauri_javelin.conf.json"; // Adjust the path as necessary
    let config = read_config(config_path)?;
    let tauri_config_path = "../src-tauri/tauri.conf.json";
    let tauri_config = read_tauri_config(tauri_config_path)?;
    let public_key = tauri_config.tauri.updater.pubkey;

    let github_username = config.github_username;
    let github_repo = config.github_repo;
    let github_pat = config.github_pat;
    let github_gist = config.gist_id;

    let operating_system = env::consts::OS;
    let architecture = env::consts::ARCH;

    let platform_key = match (operating_system, architecture) {
        ("macos", "aarch64") => "darwin-aarch64",
        ("macos", _) => "darwin-x86_64",
        ("linux", "x86_64") => "linux-x86_64",
        ("windows", "x86_64") => "windows-x86_64",
        _ => panic!(
            "Unsupported platform: {}-{}",
            operating_system, architecture
        ),
    };
    println!("OS  : {}", operating_system);
    println!("Arch  : {}", architecture);
    println!("Platform Key : {}", platform_key);

    println!("\n");
    // println!("{:?}", config); // For debugging purposes
    println!("-[Config Settings]-");

    println!("Git Username : {}", github_username);
    println!("Git Repo : {}", github_repo);
    println!("Git Gist url: {}", github_gist);
    println!("Git PAT : {}", github_pat);
    println!("Signing Secret Key : {}", config.secret_key_location);
    println!("Signing Key Password : {}", config.secret_key_password);

    println!("\n");
    // Proceed with other tasks like version increment, building the app, etc.
    println!("-[Tauri Config]-");
    println!("Product Name : {:?}", tauri_config.package.productName);
    println!("Version : {:?}", tauri_config.package.version);

    println!("\n");

    let current_version = tauri_config.package.version; // Use the version from tauri_config

    println!("Enter update type (number):\n[1] Major\n[2] Minor\n[3] Patch\n[4] Current\n[q] Quit");
    let mut update_type_str = String::new();
    io::stdin()
        .read_line(&mut update_type_str)
        .expect("Failed to read line");
    let update_type = match update_type_str.trim().to_lowercase().as_str() {
        "1" => UpdateType::Major,
        "2" => UpdateType::Minor,
        "3" => UpdateType::Patch,
        "4" => UpdateType::Current,
        "q" => std::process::exit(1),
        _ => {
            println!("Invalid update type. Please enter 'major', 'minor', or 'patch'.");
            return Ok(()); // Correctly return from the function
        }
    };

    println!(
        "Please type your update notes for the {:?} update...",
        &update_type
    );
    let mut update_notes_str = String::new();
    io::stdin()
        .read_line(&mut update_notes_str)
        .expect("Failed to read line");

    update_notes_str.trim().to_string();

    let new_version = read_and_update_version(tauri_config_path, update_type)?;
    let mut sig_content = String::new();

    // Attempt to expand the home directory in the path
    let secret_key_path = shellexpand::tilde(&config.secret_key_location).into_owned();
    let secret_key_content =
        fs::read_to_string(secret_key_path).expect("Failed to read secret key file");

    // println!("Secret Key : {}",secret_key_content);

    env::set_var("TAURI_PRIVATE_KEY", secret_key_content.trim());
    env::set_var("TAURI_KEY_PASSWORD", config.secret_key_password);

    // Retrieving and printing the environment variable to validate it
    match env::var("TAURI_PRIVATE_KEY") {
        Ok(value) => {
            let first_five = value.chars().take(5).collect::<String>();
            println!("TAURI_PRIVATE_KEY is set to: {}**********", first_five);
        }
        Err(e) => println!("Couldn't read TAURI_PRIVATE_KEY: {}", e),
    }

    //

    println!("Starting build...");

    let current_dir = env::current_dir()?;

    // Run `tauri build` command
    let output = Command::new("tauri")
        .arg("build")
        .current_dir("..") //Need to remove escape if installed as CLT
        .output()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("Success: {}", stdout);

        // Construct the path to the signature file // Need to change this after install as CLT
        let sig_file_path = format!(
            "../src-tauri/target/release/bundle/macos/{}.app.tar.gz.sig",
            tauri_config.package.productName
        );
        println!("Attempting to read Signature file path : {}", sig_file_path);
        // Read the signature file
        sig_content = fs::read_to_string(&sig_file_path).expect("Failed to read signature file");
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("\nError during build process: {}", stderr);
        println!("Ending operation, please fix the error above");
        
        exit_with_error!(config_path,&current_version);
    }

    // Change back to the original directory if needed
    env::set_current_dir(current_dir)?;

    // At this point we have all required variables and applicaiton is built can begin github api actions

    // Create release
    let operating_system = env::consts::OS;
    println!("Current Operating System : {}", operating_system);
    // Construct the path to the signature file // Need to change this after install as CLT
    let bundle_filepath = match operating_system {
        "macos" => format!(
            "../src-tauri/target/release/bundle/macos/{}.app.tar.gz",
            tauri_config.package.productName
        ),
        "windows" => format!(
            "..\\src-tauri\\target\\release\\bundle\\msi\\{}.msi.zip",
            tauri_config.package.productName
        ),
        "linux" => format!(
            "../src-tauri/target/release/bundle/appimage/{}.AppImage.tar.gz", // Assuming you're using deb for Linux
            tauri_config.package.productName
        ),
        _ => panic!("Unsupported operating system: {}", operating_system),
    };
    println!("Bundle filepath: {}", bundle_filepath);

    let new_filepath = match operating_system {
        "macos" => format!(
            "../src-tauri/target/release/bundle/macos/{}-{}.app.tar.gz",
            tauri_config.package.productName, platform_key
        ),
        "windows" => format!(
            "..\\src-tauri\\target\\release\\bundle\\msi\\{}-{}.msi.zip",
            tauri_config.package.productName, platform_key
        ),
        "linux" => format!(
            "../src-tauri/target/release/bundle/appimage/{}-{}.AppImage.tar.gz", // Assuming you're using deb for Linux
            tauri_config.package.productName, platform_key
        ),
        _ => panic!("Unsupported operating system: {}", operating_system),
    };

    // Rename the file
    fs::rename(&bundle_filepath, &new_filepath).expect("Failed to rename the file");

    println!("Artifact renamed to: {}", new_filepath);

    let filename = Path::new(&new_filepath);

    let github_user_repo = format!("{}/{}", github_username, github_repo);

    println!("GitHub User/Repo : {}",github_user_repo);

    let release_notes = update_notes_str.trim().to_string();


    let release =
        get_latest_release(&github_user_repo, &new_version, &release_notes, &github_pat).await?;

    let release_asset_url =
        upload_release_asset(&release.upload_url, filename, &github_pat).await?;

    let current_time = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let new_platform_detail = PlatformDetail {
        signature: sig_content.to_string(),
        url: release_asset_url.to_string(),
    };

    // Assuming `config` is an instance of your Config struct
    if !github_gist.trim().is_empty() {
        println!("gist_id exists and is not empty: {}", github_gist);
        // Proceed with operations that require a valid gist_id
        if let Err(e) = fetch_and_update_gist(
            &github_repo,
            &client,
            &github_pat,
            &github_gist,
            &new_version,
            &update_notes_str,
            &current_time,
            platform_key,
            new_platform_detail,
            tauri_config_path,
        )
        .await
        {
            eprintln!("Error updating gist: {}", e);
            exit_with_error!(config_path,&current_version);
        } else {
            println!("Gist updated successfully");
        }
    } else {
        println!("gist_id is empty or not set");
        // Handle the case where gist_id is empty or not set
        let gist_content = GistContent {
            version: new_version.to_string(),
            notes: update_notes_str.trim().to_string(),
            pub_date: current_time,
            platforms: {
                let mut platforms = HashMap::new();
                platforms.insert(platform_key.to_string(), new_platform_detail);
                platforms
            },
        };

        let gist_id_result = create_and_upload_gist(
            &github_repo,
            &github_username,
            &github_pat,
            &gist_content,
            platform_key,
            tauri_config_path,
        )
        .await;

        match gist_id_result {
            Ok(gist_id) => {
                println!("Gist was successfully created with ID: {}", gist_id);
                let key_path = ["gist_id"];
                if let Err(e) = update_entry_in_config(config_path, &key_path, &gist_id) {
                    eprintln!("Error updating configuration: {}", e);
                    exit_with_error!(config_path,&current_version);
                } else {
                    println!("Configuration updated successfully.");
                }
            }
            Err(e) => {
                eprintln!("Error creating gist: {}", e);

                exit_with_error!(config_path,&current_version);
            }
        }
    }

    // if let Err(e) = update_gist(&github_pat, &github_gist, &gist_content).await {
    //     eprintln!("Error updating gist: {}", e);
    // }

    println!("Updated Version to : {:?}", new_version.to_string());

    println!("\n-End of process -\n");
    Ok(())
}

async fn fetch_and_update_gist(
    github_repo: &str,
    client: &Client,
    token: &str,
    gist_id: &str,
    new_version: &str,
    new_notes: &str,
    new_pub_date: &str,
    platform_key: &str,
    new_platform_detail: PlatformDetail,
    tauri_config_path: &str,
) -> Result<(), Box<dyn Error>> {
    // Fetch the gist
    let gist_url = format!("https://api.github.com/gists/{}", gist_id);
    let response = client
        .get(&gist_url)
        .header("User-Agent", "tauri_javelin")
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
                .header("User-Agent", "tauri_javelin")
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

// https://api.github.com/repos/fleebee/fdx-auto-tauri/releases
// https://api.github.com/repos/fleebee/fdx-tauri/releases
