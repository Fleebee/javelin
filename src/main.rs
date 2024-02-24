use chrono::Utc;

use std::collections::HashMap;
use std::io::{self};
use std::process::Command;
use std::{env, fs, path::Path};

mod utilities;
use utilities::UpdateType;
use utilities::{
    read_and_update_version, read_config, read_tauri_config, reset_version_in_config,
    update_entry_in_config,
};

mod github;
use github::{
    create_and_upload_gist, fetch_and_update_gist, get_latest_release, upload_release_asset,
};
use github::{GistContent, PlatformDetail};

// Assuming the Release struct and create_github_release function are defined elsewhere

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config_path = "javelin.conf.json"; // Adjust the path as necessary
    let config = read_config(config_path)?;
    let tauri_config_path = "../src-tauri/tauri.conf.json";
    let tauri_config = read_tauri_config(tauri_config_path)?;
    // let public_key = tauri_config.tauri.updater.pubkey;

    let github_username = config.github_username;
    let github_repo = config.github_repo;
    let github_pat = config.github_pat;
    let github_gist = config.gist_id;

    let operating_system = env::consts::OS;
    let architecture = env::consts::ARCH;

    println!("OS  : {}", operating_system);
    println!("Arch  : {}", architecture);

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

    // Trim the input and check if it's empty
    let update_notes_str = update_notes_str.trim();
    let update_notes_str = if update_notes_str.is_empty() {
        // If the input is empty, use a default value
        "Routine bug fixes and performance updates"
    } else {
        // If the input is not empty, use the input value
        update_notes_str
    };
    // Use `update_notes_str` as needed from here
    println!("Update notes: {}", update_notes_str);
    println!("--------");

    let new_version = read_and_update_version(tauri_config_path, update_type)?;

    #[allow(unused_assignments)]
    let mut sig_content = String::new();

    // Attempt to expand the home directory in the path

    println!("\nResolving Secret Key:");

    let secret_key_path = match operating_system {
        "macos" | "linux" => shellexpand::tilde(&config.secret_key_location).into_owned(),
        "windows" => config.secret_key_location.clone(),
        _ => panic!("Unsupported platform"),
    };

    println! {"Secret Key PATH set as : {}",&secret_key_path};

    let secret_key_content =
        fs::read_to_string(secret_key_path).expect("Failed to read secret key file");

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

    println!("\nStarting build...");

    let current_dir = env::current_dir()?;

    let output = if cfg!(target_os = "windows") {
        println!("Os Check : Windows");
        println!("Building. This may take some time...");
        // On Windows, use `cmd /c` to run `npm run tauri build`
        Command::new("cmd")
            .args(["/C", "npm run tauri", "build"])
            .current_dir("..")
            .output()?
    } else {
        println!("Os Check : MacOs or Linux");
        println!("Building. This may take some time...");

        // Directly use `tauri` command on other operating systems
        Command::new("tauri")
            .arg("build")
            .current_dir("..")
            .output()?
    };

    if output.status.success() {
        let _stdout = String::from_utf8_lossy(&output.stdout);
        // println!("\nBuild Success: {}\n", stdout);
        println!("\nBuild Success!\n");

        println!("Constructing Signature file PATH...");

        #[cfg(target_os = "windows")]
        let sig_file_path = format!(
            "..\\src-tauri\\target\\release\\bundle\\msi\\{}_{}_x64_en-US.msi.zip.sig",
            tauri_config.package.productName, &new_version
        );

        #[cfg(target_os = "macos")]
        let sig_file_path = format!(
            "../src-tauri/target/release/bundle/macos/{}.app.tar.gz.sig",
            tauri_config.package.productName
        );

        // Similarly, you can add paths for other OSes as needed

        println!("Attempting to read Signature file path : {}", sig_file_path);
        // Read the signature file
        sig_content = fs::read_to_string(&sig_file_path).expect("Failed to read signature file");
        println!("Signature file read successfully ");
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("\nError during build process: {}", stderr);
        println!("Ending operation, please fix the error above");

        exit_with_error!(config_path, &current_version);
    }

    // Change back to the original directory if needed
    env::set_current_dir(current_dir)?;

    // At this point we have all required variables and applicaiton is built can begin github api actions

    // Create release
    println!("\nCreating Release");
    let operating_system = env::consts::OS;
    println!("Current Operating System : {}", operating_system);
    // Construct the path to the signature file // Need to change this after install as CLT
    let bundle_filepath = match operating_system {
        "macos" => format!(
            "../src-tauri/target/release/bundle/macos/{}.app.tar.gz",
            tauri_config.package.productName
        ),
        "windows" => format!(
            "..\\src-tauri\\target\\release\\bundle\\msi\\{}_{}_x64_en-US.msi.zip",
            tauri_config.package.productName, &new_version
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

    let _asset_filename = match operating_system {
        "macos" => format!("{}.app.tar.gz", tauri_config.package.productName),
        "windows" => format!(
            "{}_{}_x64_en-US.msi.zip",
            tauri_config.package.productName, new_version
        ),
        "linux" => format!(
            "{}.AppImage.tar.gz", // Assuming you're using deb for Linux
            tauri_config.package.productName
        ),
        _ => panic!("Unsupported operating system: {}", operating_system),
    };

    // Rename the file
    fs::rename(&bundle_filepath, &new_filepath).expect("Failed to rename the file");

    println!("Artifact renamed to: {}", new_filepath);

    let filename = Path::new(&new_filepath);

    let github_user_repo = format!("{}/{}", github_username, github_repo);

    println!("GitHub User/Repo : {}", github_user_repo);

    let release_notes = update_notes_str.trim().to_string();

    println!("Fetching latest release");
    let release =
        get_latest_release(&github_user_repo, &new_version, &release_notes, &github_pat).await?;

    // TODO Add check for asset filename in existing release
    // TODO Add fn to delete existing asset if exists
    // - This is necessary to prevent upload failure

    println!("Uploading Release...");
    let release_asset_url =
        upload_release_asset(&release.upload_url, filename, &github_pat).await?;

    println!("\nResolving Gist Data");
    let current_time = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let new_platform_detail = PlatformDetail {
        signature: sig_content.to_string(),
        url: release_asset_url.to_string(),
    };

    if !github_gist.trim().is_empty() {
        println!("gist_id exists and is not empty: {}", github_gist);
        // Proceed with operations that require a valid gist_id
        if let Err(e) = fetch_and_update_gist(
            &github_repo,
            &github_pat,
            &github_gist,
            &new_version,
            update_notes_str,
            &current_time,
            platform_key,
            new_platform_detail,
        )
        .await
        {
            eprintln!("Error updating gist: {}", e);
            exit_with_error!(config_path, &current_version);
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
                    exit_with_error!(config_path, &current_version);
                } else {
                    println!("Configuration updated successfully.");
                }
            }
            Err(e) => {
                eprintln!("Error creating gist: {}", e);

                exit_with_error!(config_path, &current_version);
            }
        }
    }

    println!("Updated Version to : {:?}", new_version.to_string());

    println!("\n-End of process -\n");
    Ok(())
}
