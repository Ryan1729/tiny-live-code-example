extern crate libloading;

use libloading::Library;

#[cfg(all(debug_assertions, unix))]
const LIB_PATH: &'static str = "./target/debug/libstate_manipulation.so";
#[cfg(all(not(debug_assertions), unix))]
const LIB_PATH: &'static str = "./target/release/libstate_manipulation.so";

#[cfg(all(debug_assertions, windows))]
const LIB_PATH: &'static str = "./target/debug/state_manipulation.dll";
#[cfg(all(not(debug_assertions), windows))]
const LIB_PATH: &'static str = "./target/release/state_manipulation.dll";

struct Application {
    library: Library,
}

impl Application {
    fn new() -> Self {
        let library = Library::new(LIB_PATH).unwrap_or_else(|error| panic!("{}", error));

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
    let mut app = Application::new();

    let mut counter = 0;

    let mut last_modified = std::fs::metadata(LIB_PATH).unwrap().modified().unwrap();

    app.update_and_render(&mut counter);

    let frame_duration = std::time::Duration::new(0, 2000000000);

    loop {
        let start = std::time::Instant::now();

        app.update_and_render(&mut counter);

        if let Ok(Ok(modified)) = std::fs::metadata(LIB_PATH).map(|m| m.modified()) {
            println!("was: {:?}", last_modified);
            println!("now: {:?}", modified);
            if modified > last_modified {
                drop(app);
                app = Application::new();
                last_modified = modified;
            }
        }

        if let Some(sleep_time) =
            frame_duration.checked_sub(std::time::Instant::now().duration_since(start))
        {
            std::thread::sleep(sleep_time);
        }
    }
}
