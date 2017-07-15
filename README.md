# Live Code SDL2 OpenGL 2.1 Template

This is a template crate to make it easy to start a new project using SDL2 and OpenGL 2.1, with live code reloading.

Similarly to the way [live-code-bear-lib-terminal-template](https://github.com/Ryan1729/live-code-bear-lib-terminal-template) turned out, the plan is to use this template in projects, then backport generally useful additions to the `Platform` struct, expanding that API based on actuall usage, rather than blind guessing.

## Compiling release mode

Comment out the line containing `crate-type = ["dylib"]` in the `Cargo.toml` in the `state_manipulation` folder. It would be nice if this step was eliminated, but AFAIK the only way to do that would be to build both the `dylib` and the `rlib` every time. Given how relatively rare release builds are, that seems like a waste of compile time.

Then run `cargo build --release`
