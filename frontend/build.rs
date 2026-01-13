use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs");

    if let Ok(output) = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        && output.status.success()
    {
        let hash = String::from_utf8_lossy(&output.stdout);
        println!("cargo:rustc-env=GIT_HASH={}", hash.trim());
    }
}
