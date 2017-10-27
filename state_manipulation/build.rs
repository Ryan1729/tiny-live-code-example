use std::env;
use std::path::Path;
use std::process::Command;
use std::fs;

fn main() {
    let profile = env::var("PROFILE").unwrap_or("debug".to_string());

    if profile == "debug" {
        let current_dir = std::env::current_dir().unwrap();
        let current_parent = current_dir.parent().unwrap();
        let target = Path::new(&current_parent).join("target/debug");

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

        // rlib number instructions:
        //to update the rlib number, comment out everything below this
        //and run `cargo build -vv` and note what is passed to rustc
        //while building state_manipulation. Specifically look for
        //something like the string below and change the number
        //(between "-" and ".rlib" below), to match.
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
            Ok(output) => match output.status.code() {
                Some(101) => {
                    let message = format!(
                        "If the error below is complaining about dependancies not being found, \
                         or that they were compiled with an incompatible version of rustc, \
                         then the rlib number may have changed. \
                         In that case, \
                         open the {} build.rs and follow the rlib number instructions.",
                        crate_name
                    );
                    panic!("\n\n{}\n\n {:?}", message, output)
                }
                Some(0) => {}
                _ => panic!("{:?}", output),
            },
        }
    }
}
