use std::{process::Command, time::{SystemTime, UNIX_EPOCH}};

fn command_output(program: &str, args: &[&str]) -> String {
    Command::new(program).args(args).output().ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|output| !output.is_empty())
        .unwrap_or_else(|| "unknown".into())
}

fn main() {
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/index");
    println!("cargo:rerun-if-changed=../.git/refs/heads/main");
    println!("cargo:rustc-env=RENDERNORTH_GIT_COMMIT={}", command_output("git", &["rev-parse", "--short=12", "HEAD"]));
    println!("cargo:rustc-env=RENDERNORTH_RUST_VERSION={}", command_output("rustc", &["--version"]));
    let build = SystemTime::now().duration_since(UNIX_EPOCH).map(|value| value.as_secs()).unwrap_or(0);
    println!("cargo:rustc-env=RENDERNORTH_BUILD_UNIX={build}");
    tauri_build::build()
}
