# Javelin for TAURI

### https://github.com/Fleebee/javelin

## Summary

A tool to automate github version releases for Tauri applications.
Handles version number increment by update type, building and deployment to Github Releases.

### TLDR :

- run the tool
- select update type
- describe version changes
- builds and deploys to github Releases / Gist while you drink a coffee

## The Problem

You've spent 20 hours adding a new quality of life feature. You're excited to release it to the masses but now you have to:

- Update the version number in tauri.conf
- Draft the release in Github
- Update/Create the manifest in Gist
- Add the Gist URL to tauri.conf
- Build the Tauri project, copying your secret key to use
- Upload the assets to Github Releases and copy the package url
- Copy the signature file contents
- Go back to Gists and update the JSON file with the package url and signature key

Got more than one platform to deploy to? Get ready to do it all again.

## The Solution

A command line tool that will ask you for the update type (Major, Minor, Patch) and a description and handle the rest for you.

### The complete automated steps are:

- Read configuration from the javelin.conf.json file
- Increment the version number of the application by defined type in the CLI
- Obtain the secret key and password from a defined filepath on host machine and write into the system ENV
- Trigger the tauri build command
- Obtain the contents of the signature file from the created bundle directory
- Upload the release to 'Github releases' with tag and description
- obtain the release file url
- update or create a Github gist static json file with : Version, System Arch, release url, signature, release notes

## Pre-requisites

- [Required] The package should be run from the root directory of a Tauri application
- [Required] The Tauri project should have an existing git repo
- [Optional] An existing Gist code/id
- [Required] You must have a Git PAT key
- [Required] You must have generated a secret and public signing key in accordance with the Tauri documentation: https://tauri.app/v1/guides/distribution/updater/

## Instructions

### Setup (Rust files)

- The repo should be cloned into the root dir of your Tauri project, next to src-tauri
- Inside the javelin folder , rename sample_javelin.conf.json to javelin.conf.json
- All fields in javelin.conf.json are required except for gist_id - this will be created if blank
- You must create a key pair [secret/pub] you can do this by following the instructions in the Tauri docs for Updater
- Any required field not filled at execution will be prompted for input in the CLI

### Setup (Installed)
- Install the Javelin package and add to source
- Run with the command 'Javelin' in the root dir of your TAURI project
- On first run a javelin.conf.json file will be created and prompt for values

{
  "gist_id": "", // Leave blank unless you have an existing Gist manifest, this wil be created
  "github_pat": "", // Your Github auth key
  "github_repo": "", // The Tauri project repo name (Not url)
  "github_username": "", // Your Github Username
  "secret_key_location": "", // Path to generated .key file generated according to the Tauri docs
  "secret_key_password": "", // The password to your key file, leave blank if none
}

### Usage

- From a terminal while in the javelin dir, run 'cargo run' (or from root dir of Tauri project type Javelin if not running from Rust files)
- Type the type of update you will be performing and press Enter, this will increae a digit in the version number
- Type your update description and press enter - this is added to the Release description and Gist

### Output

- The application will run the build command automatically
- The version number in your tauri.conf.json file will be incremented
- The github release will be created and your bundle file uploaded and gist will be created and populated with System OS, signing key and Release url
- The Gist ID will be added to javelin.conf.json and the full Gist url will be added to [tauri.conf.json][updater]
- endpoints should be automatically set to ["https://gist.github.com/{YOUR_GIT_USERNAME}/{YOUR_GIST_ID}/raw"]
- The release will be available to your users (this may take a minute or two to propogate)
- Errors should show in the terminal output if any

## Considerations

- This tool will push to Private repos with a valid PAT key, but your deployed applicaiton will not be able to download from a private repo. This should be handled by overiding the Bearer Header in your Tauri application.
- Each OS type will create an individual manifest.json file but can share Release versions nd upload their own artifacts. This is because there were issues when the manifest version was updated for a platform, every platform considered there to be a new version. It can be done but considering the manifest wouldnt track all version numbers it became less important to fix it.
- If no Gist key is inputted a draft will be created and populated with Release details

## Known issues

- If you try to deploy an existing version number for an existing OS, you will get an error Status: 422 Unprocessable Entity. This can be fixed in future with a delete function for the existing asset, for now you can either manually remove the asset in Github releases, or deploy a different version number.


### Please note, this application is not created by or endorsed by TAURI. It is intended for use to automate some deployment tasks.

### “TAURI is a trademark of The Tauri Programme within the Commons Conservancy. [https://tauri.app/]”
