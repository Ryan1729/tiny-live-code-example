use std::env;
use std::path::Path;
use std::fs;

fn main() {
    let profile = env::var("PROFILE").unwrap_or("debug".to_string());

    if profile == "debug" {
        let current_dir = std::env::current_dir().unwrap();
        let reloaded = Path::new(&current_dir).join("target/debug/reloaded");

        //this just slows the accumulation of old versions of reloaded modules
        fs::remove_dir_all(&reloaded).unwrap_or(());
    }
}
