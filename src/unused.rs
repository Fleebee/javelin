// Models

#[derive(Debug, Deserialize)]
struct ReleaseGet {
    id: i64,
    tag_name: String,
    name: String,
    body: Option<String>,
}



#[derive(Debug, Serialize, Deserialize)]
pub struct GistFile {
    filename: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Gist {
    files: HashMap<String, GistFile>,
}



// Functions

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