For a release I need to:

- update the Version number in `Cargo.toml`
- make new commit with these changes
- tag that commit with the version number
- push tags to github (git push --tags, though annotated tags are allegedly better)
- Write the Changelog in the release on Github
- optionally compile for more targets locally and upload those to Github
- compile for wasm
- copy the code to the server
- update the Version number on the server!!
