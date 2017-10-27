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

        let target_string = target.to_str().unwrap();

        // rlib number instructions:
        //to update the rlib number, comment out everything below this
        //and run `cargo build -vv` and note what is passed to rustc
        //while building state_manipulation. Specifically look for
        //something immeadiately after an `--extern` flag that looks
        //like the string below and change the number (the part between
        //"-" and ".rlib" below), to match, or add an entry to this
        //vector if there is an unaccounted for `--extern` flag.
        let dependancies = vec![
            format!(
                "common={}/deps/libcommon-4235dfc929cd5f11.rlib",
                target_string
            ),
        ];
        let r = {
            let mut c = Command::new("rustc");
            c.arg(&format!("{}/src/lib.rs", current_dir.to_str().unwrap()))
                .arg("--crate-name")
                .arg(lib_name)
                .arg("--crate-type")
                .arg("dylib")
                .arg("-L")
                .arg(format!("dependency={}/deps", target_string))
                .arg("--out-dir")
                .arg(reloaded);

            for dependacy in dependancies {
                c.arg("--extern").arg(dependacy);
            }

            c.output()
        };

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
