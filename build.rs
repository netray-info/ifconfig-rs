use std::path::Path;

fn main() {
    println!("cargo::rerun-if-changed=frontend/dist");

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
