# Javelin for TAURI

## Summary

This tool was created to assist in github version releases.
It is a command line tool for automatic building, deploying and updating the manifest of a TAURI application.

## Aim

I wanted a method to use Github to host the releases and manifest but didnt like the task of signing, manual uploads and release creation each time.
The work I do requires multi plat always , so needed something painless to specify Major, Minor, Patch updates


### The automated steps are:

- Read configuration from the javelin.conf.json file
- Increment the version number of the application by defining the update type in the CLI
- Obtain the secret key and password from a defined filepath on host machine and write into the system ENV
- Trigger the tauri build command
- Obtain the contents of the signature file from the created bundle directory
- Upload the release to 'Github releases' with tag and description
- obtain the release file url
- update or create a Github gist static json file with : Version, System Arch, release url, signature, release notes

## Pre-requisites

- [Required] The package should be run from the root directory of a Tauri application
- [Required] The Tauri project should have an existing git repo
- [Optional] An existing Gist code
- [Required] You must have a Git PAT key
- [Required] You must have generated a secret and public key in accordance with the Tauri documentation

## Instructions

- The repo should be cloned into the root dir of your Tauri project, next to src-tauri
- Inside the tauri_javelin folder , rename sample_javelin.conf.json to javelin.conf.json
- All fields in tauri_javelin.conf.json are required except for gist_id - this will be created if blank
- You must create a key pair [secret/pub] you can do this by following the instructions in the Tauri docs for Updater
- From a terminal while in the javelin dir, run 'cargo run'
- Type the type of update you will be performing and press Enter, this will increae a digit in the version number
- Type your update description and press enter - this is added to the Release description and Gist

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

## Known issues

- If you try to deploy an existing version number for an existing OS, you will get an error Status: 422 Unprocessable Entity
- This can be fixed in future with a delete function for the existing asset, for now you can either manually remove the asset in Github releases, or deploy a different version number.
- Other issues to be added...


### Please note, this application is not created by or endorsed by TAURI. It is intended for use to automate some deployment tasks.
### “TAURI is a trademark of The Tauri Programme within the Commons Conservancy. [https://tauri.app/]”
