use serde::Deserialize;
use serde_json::{Value,json};
use std::error::Error;
use std::{fs,fs::File, path::Path};
use std::io::{self, Write};


pub fn create_default_config_if_not_exists(config_path: &str) -> Result<(), io::Error> {
    // Check if the file already exists
    if Path::new(config_path).exists() {
        println!("Config file found");
        return Ok(());
    }

    // Create a JSON object with the default configuration
    let default_config = json!({
        "gist_id": "",
        "github_pat": "",
        "github_repo": "",
        "github_username": "",
        "secret_key_location": "",
        "secret_key_password": "",
    });

    // Open the file in write mode and write the JSON content to it
    let mut file = File::create(config_path)?;
    file.write_all(serde_json::to_string_pretty(&default_config).unwrap().as_bytes())?;

    println!("Config file created at {}", config_path);

    Ok(())
}

pub fn read_value(prompt: &str, value: &mut String) {
    if value.trim().is_empty() {
        print!("{} empty, Enter {}: ", prompt, prompt);
        io::stdout().flush().unwrap();
        io::stdin().read_line(value).expect("Failed to read input");
    }
    *value = value.trim().to_string(); // Remove trailing newline
}

pub fn update_tauri_config_endpoint(
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
#[macro_export]
macro_rules! exit_with_error {
    ($config_path:expr, $current_version:expr) => {{
        println!("Error occurred in file: {}, line: {}", file!(), line!());
        let _result = reset_version_in_config($config_path, $current_version);
        std::process::exit(1);
    }};
}

pub fn read_and_update_version<P: AsRef<Path>>(
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

pub fn update_version(
    current_version: &str,
    update_type: UpdateType,
) -> Result<String, &'static str> {
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
            segments[2] += 0; // Increment patchf
        }
    }

    Ok(format!("{}.{}.{}", segments[0], segments[1], segments[2]))
}

pub fn reset_version_in_config(
    config_path: &str,
    reset_version: &str,
) -> Result<(), Box<dyn Error>> {
    // Read the current configuration
    let config_contents = fs::read_to_string(config_path)?;
    let mut config: Value = serde_json::from_str(&config_contents)?;

    // Assuming the version is under "package" object
    if let Some(package) = config["package"].as_object_mut() {
        if let Some(version) = package.get_mut("version") {
            match version {
                Value::String(version_str) => *version_str = reset_version.to_string(),
                _ => return Err("Failed to update version: 'version' field is not a string".into()),
            }
        } else {
            return Err("Failed to update version: 'version' field not found".into());
        }
    } else {
        return Err("Failed to update version: 'package' object not found".into());
    }

    // Write the updated configuration back to the file
    fs::write(config_path, serde_json::to_vec_pretty(&config)?)?;

    Ok(())
}

pub fn update_entry_in_config(
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

pub fn read_config<P: AsRef<Path>>(path: P) -> Result<Config, Box<dyn std::error::Error>> {
    let config_str = fs::read_to_string(path)?;
    let config: Config = serde_json::from_str(&config_str)?;
    Ok(config)
}

pub fn read_tauri_config<P: AsRef<Path>>(
    path: P,
) -> Result<TauriConfig, Box<dyn std::error::Error>> {
    let tauri_config_str = fs::read_to_string(path)?;
    let tauri_config: TauriConfig = serde_json::from_str(&tauri_config_str)?;
    Ok(tauri_config)
}

#[derive(Deserialize, Debug)]
pub struct TauriConfig {
    pub package: Package,
    pub tauri: Tauri,
}
#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
pub struct Package {
    pub productName: String,
    pub version: String,
}

#[derive(Deserialize, Debug)]
pub struct Tauri {
    pub updater: Updater,
}

#[derive(Deserialize, Debug)]
pub struct Updater {
    pub pubkey: String,
    pub endpoints: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub github_username: String,
    pub github_repo: String,
    pub github_pat: String,
    pub secret_key_location: String,
    pub secret_key_password: String,
    pub gist_id: String,
}

#[derive(Debug)]
pub enum UpdateType {
    Major,
    Minor, // Using Minor instead of Feature for conventional naming
    Patch,
    Current,
}
