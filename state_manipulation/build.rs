use std::env;
use std::path::Path;
use std::process::Command;
use std::fs;

fn main() {
    let profile = env::var("PROFILE").unwrap_or("Debug".to_string());
    let current_dir = std::env::current_dir().unwrap();
    let current_parent = current_dir.parent().unwrap();
    let target;

    if profile == "Release" {
        target = Path::new(&current_parent).join("target/release");
    } else {
        target = Path::new(&current_parent).join("target/debug");
    }

    let reloaded = target.join("reloaded");

    fs::create_dir_all(&reloaded).unwrap();

    let crate_name = "state_manipulation";

    let mut count = 0;

    for entry in fs::read_dir(&reloaded).unwrap() {
        let path = entry.unwrap().path();
        if !path.is_dir() && path.to_str().unwrap().contains(crate_name) {
            count += 1;
        }
    }

    let lib_name = &format!("{}{}", crate_name, count);

    //to update this, comment out everything blow this
    //and run `cargo build -vv` and note waht is passed
    //to rustc while building state_manipulation
    let common = &format!(
        "common={}/deps/libcommon-4235dfc929cd5f11.rlib",
        target.to_str().unwrap()
    );

    let r = Command::new("rustc")
        .arg(&format!("{}/src/lib.rs", current_dir.to_str().unwrap()))
        .arg("--crate-name")
        .arg(lib_name)
        .arg("--crate-type")
        .arg("dylib")
        .arg("--out-dir")
        .arg(reloaded)
        .arg("--extern")
        .arg(common)
        .output();

    match r {
        Err(e) => panic!("failed to execute process: {}", e),
        Ok(_) => {}
        // Ok(o) => panic!("{:?}", o),
    }
}
