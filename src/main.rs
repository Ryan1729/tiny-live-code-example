extern crate libloading;

use libloading::Library;

#[cfg(unix)]
const LIB_NAME_PREFIX: &'static str = "libstate_manipulation";
#[cfg(windows)]
const LIB_NAME_PREFIX: &'static str = "state_manipulation";

#[cfg(unix)]
const LIB_EXTENSION: &'static str = ".so";
#[cfg(windows)]
const LIB_EXTENSION: &'static str = ".dll";

#[cfg(debug_assertions)]
const LIB_PATH_PREFIX: &'static str = "./target/debug/";
#[cfg(not(debug_assertions))]
const LIB_PATH_PREFIX: &'static str = "./target/release/";

fn get_lib_path(suffix: &str) -> String {
    format!(
        "{}{}{}{}",
        LIB_PATH_PREFIX,
        LIB_NAME_PREFIX,
        suffix,
        LIB_EXTENSION
    )
}

struct Application {
    library: Library,
}

impl Application {
    fn new() -> Self {
        let library = Library::new(get_lib_path("")).unwrap_or_else(|error| panic!("{}", error));

        Application { library: library }
    }

    fn update_and_render(&self, counter: &mut i64) {
        unsafe {
            let f = self.library
                .get::<extern "C" fn(&mut i64)>(b"lib_update_and_render\0")
                .unwrap();
            f(counter)
        }
    }
}

fn main() {
    std::fs::copy(&get_lib_path("1"), &get_lib_path("")).unwrap();

    let mut app = Application::new();

    let mut counter = 0;

    let mut last_modified = std::fs::metadata(get_lib_path(""))
        .unwrap()
        .modified()
        .unwrap();

    app.update_and_render(&mut counter);

    let frame_duration = std::time::Duration::new(0, 2000);

    let mut loop_counter = 0;
    let mut not_renamed_yet = true;

    loop {
        let start = std::time::Instant::now();

        app.update_and_render(&mut counter);

        if let Ok(Ok(modified)) = std::fs::metadata(get_lib_path("")).map(|m| m.modified()) {
            println!("was: {:?}", last_modified);
            println!("now: {:?}", modified);
            if modified > last_modified {
                drop(app);
                app = Application::new();
                last_modified = modified;
            }
        }

        if loop_counter >= 10 {
            std::process::exit(if counter > 1000 {
                //success
                0
            } else {
                //failure
                1
            });
        } else if loop_counter >= 5 && not_renamed_yet {
            println!("rename");
            std::fs::rename(&get_lib_path(""), &get_lib_path("old")).unwrap();
            std::fs::copy(&get_lib_path("1000"), &get_lib_path("")).unwrap();
            not_renamed_yet = false;
        }

        loop_counter += 1;

        if let Some(sleep_time) =
            frame_duration.checked_sub(std::time::Instant::now().duration_since(start))
        {
            std::thread::sleep(sleep_time);
        }
    }
}
