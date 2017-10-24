# Tiny Live Code Example

This is an example crate demonstrating a way to do live code reloading.

![demo gif](/demo.gif?raw=true "Demo gif")

This crate is currently only tested on Linux

# How does this work?

The basic idea is that (in debug mode) we load a dynamic library, pass it a mutable reference to a chunk of state. We then check the modification time of the library periodically and swap it out if it is newer than before. Because we only gave th library code a mutable reference to the state, it is still safe and sound and the program can continue on uninterupeted if the changes aren't too drastic.

## Compiling release mode

Comment out the line containing `crate-type = ["dylib"]` in the `Cargo.toml` in the `state_manipulation` folder. It would be nice if this step was eliminated, but AFAIK the only way to do that would be to build both the `dylib` and the `rlib` every time. Given how relatively rare release builds are, that seems like a waste of compile time.

Then run `cargo build --release`
