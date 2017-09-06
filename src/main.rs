extern crate common;

#[cfg(debug_assertions)]
extern crate libloading;
#[cfg(not(debug_assertions))]
extern crate state_manipulation;

#[cfg(debug_assertions)]
use libloading::Library;

#[cfg(debug_assertions)]
use std::mem::ManuallyDrop;

use common::*;

#[cfg(all(debug_assertions, unix))]
const LIB_PATH: &'static str = "./target/debug/libstate_manipulation.so";
#[cfg(all(debug_assertions, windows))]
const LIB_PATH: &'static str = "./target/debug/state_manipulation.dll";
#[cfg(not(debug_assertions))]
const LIB_PATH: &'static str = "Hopefully compiled out";

#[cfg(debug_assertions)]
struct Application {
    library: ManuallyDrop<Library>,
}

#[cfg(debug_assertions)]
impl Drop for Application {
    fn drop(&mut self) {
        unsafe {
            ManuallyDrop::drop(&mut self.library);
        }
    }
}

#[cfg(not(debug_assertions))]
struct Application {}

#[cfg(debug_assertions)]
impl Application {
    fn new() -> Self {
        let library = Library::new(LIB_PATH).unwrap_or_else(|error| panic!("{}", error));

        Application {
            library: ManuallyDrop::new(library),
        }
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

    let frame_duration = std::time::Duration::new(0, 4000000000);

    loop {
        let start = std::time::Instant::now();

        app.update_and_render(&platform, &mut state);

        if cfg!(debug_assertions) {
            if let Ok(Ok(modified)) = std::fs::metadata(LIB_PATH).map(|m| m.modified()) {
                println!("last_modified: {:?}", last_modified,);
                println!("modified: {:?}", modified,);
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
