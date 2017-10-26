use std::env;
use std::path::Path;
use std::fs;

fn main() {
    let profile = env::var("PROFILE").unwrap_or("Debug".to_string());
    let current_dir = std::env::current_dir().unwrap();
    let target;

    if profile == "Release" {
        target = Path::new(&current_dir).join("target/release");
    } else {
        target = Path::new(&current_dir).join("target/debug");
    }

    let reloaded = target.join("reloaded");

    //this just slows the accumulation of old versions of reloaded modules
    fs::remove_dir_all(&reloaded).unwrap_or(());
}
