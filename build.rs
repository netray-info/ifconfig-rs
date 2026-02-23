use std::path::Path;

fn main() {
    println!("cargo::rerun-if-changed=frontend/dist");
    println!("cargo::rerun-if-changed=.git/HEAD");
    println!("cargo::rerun-if-changed=.git/refs/heads");

    let git_sha = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo::rustc-env=GIT_SHORT_SHA={git_sha}");

    let git_date = std::process::Command::new("git")
        .args(["log", "-1", "--format=%cd", "--date=short"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo::rustc-env=BUILD_DATE={git_date}");

    let dist = Path::new("frontend/dist/index.html");
    if !dist.exists() {
        if cfg!(debug_assertions) {
            // Create a placeholder for debug builds
            let dir = Path::new("frontend/dist");
            std::fs::create_dir_all(dir).expect("Failed to create frontend/dist");
            std::fs::write(
                dist,
                "<!DOCTYPE html><html><body><p>Run <code>cd frontend &amp;&amp; npm run build</code> for the real SPA.</p></body></html>",
            )
            .expect("Failed to write placeholder index.html");
        } else {
            panic!("frontend/dist/index.html not found — run `cd frontend && npm run build` before a release build");
        }
    }
}
