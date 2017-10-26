extern crate common;

#[cfg(debug_assertions)]
extern crate libloading;
#[cfg(not(debug_assertions))]
extern crate state_manipulation;

#[cfg(debug_assertions)]
use libloading::Library;

use common::*;

#[cfg(all(debug_assertions, unix))]
const LIB_PATH: &'static str = "./target/debug/libstate_manipulation.so";
#[cfg(all(debug_assertions, unix))]
const LIB_PREFIX: &'static str = "./target/debug/reloaded/libstate_manipulation";
#[cfg(all(debug_assertions, unix))]
const LIB_EXT: &'static str = ".so";


#[cfg(all(debug_assertions, windows))]
const LIB_PATH: &'static str = "./target/debug/state_manipulation.dll";
#[cfg(all(debug_assertions, windows))]
const LIB_PREFIX: &'static str = "./target/debug/reloaded/state_manipulation";
#[cfg(all(debug_assertions, windows))]
const LIB_EXT: &'static str = ".dll";
#[cfg(not(debug_assertions))]
const LIB_PATH: &'static str = "Hopefully compiled out";

#[cfg(debug_assertions)]
use std::path::PathBuf;

#[cfg(debug_assertions)]
struct Application {
    library: Library,
}
#[cfg(not(debug_assertions))]
struct Application {}

#[cfg(debug_assertions)]
impl Application {
    fn new() -> Self {
        fn make_path_buf(counter: usize) -> PathBuf {
            let mut result = PathBuf::new();

            result.push(format!("{}{}{}", LIB_PREFIX, counter, LIB_EXT));

            result
        }

        //TODO A binary-search-like thing could be done to make this O(log n)
        let mut counter = 0;
        let mut path = make_path_buf(counter);
        let mut new_path = make_path_buf(counter);

        println!("start by trying {}", new_path.to_str().unwrap());

        if new_path.exists() {
            while new_path.exists() {
                path = new_path;
                counter += 1;
                new_path = make_path_buf(counter);
                println!("trying {}", new_path.to_str().unwrap());
            }
        } else {
            path = PathBuf::new();
            path.push(LIB_PATH.to_string());
        }

        println!("loading {}", path.to_str().unwrap());

        let library = Library::new(path).unwrap_or_else(|error| panic!("{}", error));

        Application { library: library }
    }

    fn new_state(&self) -> State {
        unsafe {
            let f = self.library.get::<fn() -> State>(b"new_state\0").unwrap();

            f()
        }
    }

    fn update_and_render(&self, platform: &Platform, state: &mut State) -> bool {
        unsafe {
            let f = self.library
                .get::<fn(&Platform, &mut State) -> bool>(b"update_and_render\0")
                .unwrap();
            f(platform, state)
        }
    }
}
#[cfg(not(debug_assertions))]
impl Application {
    fn new() -> Self {
        Application {}
    }

    fn new_state(&self) -> State {
        state_manipulation::new_state()
    }

    fn update_and_render(&self, platform: &Platform, state: &mut State) -> bool {
        state_manipulation::update_and_render(platform, state)
    }
}

fn main() {
    let mut app = Application::new();

    let mut state = app.new_state();

    let mut last_modified = if cfg!(debug_assertions) {
        std::fs::metadata(LIB_PATH).unwrap().modified().unwrap()
    } else {
        //hopefully this is actually compiled out
        std::time::SystemTime::now()
    };

    //You can put platform-specfic fn pointers here
    let platform = Platform {};

    app.update_and_render(&platform, &mut state);

    let frame_duration = std::time::Duration::new(0, 50000000);

    loop {
        let start = std::time::Instant::now();

        app.update_and_render(&platform, &mut state);

        if cfg!(debug_assertions) {
            if let Ok(Ok(modified)) = std::fs::metadata(LIB_PATH).map(|m| m.modified()) {
                if modified > last_modified {
                    drop(app);
                    app = Application::new();
                    last_modified = modified;
                }
            }
        }

        if let Some(sleep_time) =
            frame_duration.checked_sub(std::time::Instant::now().duration_since(start))
        {
            std::thread::sleep(sleep_time);
        }
    }
}
