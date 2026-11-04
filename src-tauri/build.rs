fn main() {
    // A build id for "once per installed build" things (the full intro):
    // GIT_SHA from the preview / release workflow, else GitHub's own
    // GITHUB_SHA, else the local checkout's HEAD, else empty (the app then
    // falls back to its version).
    println!("cargo:rerun-if-env-changed=GIT_SHA");
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    let sha = std::env::var("GIT_SHA")
        .ok()
        .or_else(|| std::env::var("GITHUB_SHA").ok())
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            std::process::Command::new("git")
                .args(["rev-parse", "HEAD"])
                .output()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        })
        .unwrap_or_default();
    let short: String = sha.trim().chars().filter(|c| c.is_ascii_alphanumeric()).take(12).collect();
    println!("cargo:rustc-env=COVE_BUILD_ID={short}");
    tauri_build::build()
}
