#Tauri Javelin

##Summary
This tool was created to assist in github version releases.
It is a command line tool for automatic building, deploying and updating of a Tauri application.

#Aim
Integrate Tauri and Github as a solution to specify the version update type - Major, Minor, Patch and update the version record before uploading the artfacts and updating the update manifest.
An attempt to reduce the amount and location points of data entries and manual updates.


The steps are:
- Read configuration from the tauri_javelin.conf.json file
- Increment the version number of the application by defining the update type
- Obtain the secret key and password from a defined location and write into the system ENV
- Trigger the tauri build
- Obtain the contents of the signature file from the bundle directory
- Upload the release to 'Github releases' with tag and description
- obtain the release file url
- update the Github gist static json file with : Version, release url, signature, release notes


#Pre-requisites
- The package should be run from the root directory of a Tauri application
- The Tauri project should have an existing git repo
- There should be a gist already created  //may be able to automate this too
- You must have a Git PAT key
- You must have generated a secret and public key in accordance with the Tauri documentation


#Instructions

[tauri.conf.json][updater]
- endpoints should be set to ["https://gist.github.com/{YOUR_GIT_USERNAME}/{YOUR_GIST_ID}/raw"]
