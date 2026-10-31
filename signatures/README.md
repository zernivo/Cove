# Update signatures

Cove's in-app updater only installs a release whose `Cove_x.y.z_x64-setup.exe`
has a matching `.sig` here, signed offline with Cove's update key (minisign,
Tauri updater format). The private key is never stored on GitHub, so a
tampered release or a compromised CI run can't produce an installer Cove
will accept. The public key is built into `src-tauri/src/updater.rs`.
