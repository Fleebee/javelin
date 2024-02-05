# Tauri Javelin

## Summary

This tool was created to assist in github version releases.
It is a command line tool for automatic building, deploying and updating of a Tauri application.

## Aim

Integrate Tauri and Github as a solution to specify the version update type - Major, Minor, Patch and update the version record before uploading the artfacts and updating the update manifest.
Reduce the points of data entries and manual updates when using git to handle releases.

### The automated steps are:

- Read configuration from the tauri_javelin.conf.json file
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
- Inside the tauri_javelin folder , rename sample_tauri_javelin.conf.json to tauri_javelin.conf.json
- All fields in tauri_javelin.conf.json are required except for gist_id - this will be created if blank
- You must create a key pair [secret/pub] you can do this by following the instructions in the Tauri docs for Updater
- From a terminal while in the tauri_javelin dir, run 'cargo run'
- Type the type of update you will be performing and press Enter, this will increae a digit in the version number
- Type your update description and press enter - this is added to the Release description and Gist

- The application will run the build command automatically
- The version number in your tauri.conf.json file will be incremented
- The github release will be created and your bundle file uploaded and gist will be created and populated with System OS, signing key and Release url
- The Gist ID will be added to tauri_javelin.conf.json and the full Gist url will be added to [tauri.conf.json][updater]
- endpoints should be automatically set to ["https://gist.github.com/{YOUR_GIT_USERNAME}/{YOUR_GIST_ID}/raw"]
- The release will be available to your users (this may take a minute or two to propogate)
- Errors should show in the terminal output if any

## Considerations

- This tool will push to Private repos with a valid PAT key, but your deployed applicaiton will not be able to download from a private repo. This should be handled by overiding the Bearer Header in your Tauri application.

## Known issues

- If you try to deploy an existing version number for an existing OS, you will get an error Status: 422 Unprocessable Entity
- This can be fixed in future with a delete fn for the existing asset, for now you can either manually remove the asset in Fithub releases, or deploy a different version number.
- Other issues to be added...
