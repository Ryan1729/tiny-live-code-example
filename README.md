# Tiny Live Code Example

This is an example crate demonstrating a way to do live code reloading.

![demo gif](/demo.gif?raw=true "Demo gif")

This crate is currently only tested on Linux, but an attempt has been made to be portable, so it might work elsewhere.

# How does this work?

The basic idea is that (in debug mode) we load a dynamic library, pass it a mutable reference to a chunk of state. We then check the modification time of the library periodically and swap it out if it is newer than before. Because we only gave th library code a mutable reference to the state, it is still safe and sound and the program can continue on uninterupeted if the changes aren't too drastic.

Currently, in order to work around some operating system calls not reloading a library compiled with the same crate name, we are compiling an additional version of the reloaded library in the build script everytime that library changes. The extra copies are placed in `target/debug/reloaded/` and the highest consecutative library is loaded from there.

## "the rlib number may have changed."?

As mentioned in the previous paragraph we are compiling an extra version of the reloaded library in the build script. As of now, (Late October 2017,) `cargo` does not provide a nice way to get the flags that it will pass to `rustc`, make a small change, (in our case the crate name and output location,) and pass the updated flags to `rustc` ourselves. So, in order to link in the reloaded libraries dependancies we need to know the name of the file(s) of the intermeadiate outputs so we can use them in our extra copy. These filenames look like this: `libcommon-4235dfc929cd5f11.rlib`. The number seems to stay stable if the same version of the compiler is used. So, until `cargo` supports things like this better, (and the responses in [this thread](https://internals.rust-lang.org/t/what-do-rust-tools-need-from-the-build-system/5975) seem to indicate desire for this from enough people that it will probably happen eventually,) occasionally you may get this error, which will give further details about what to do.

## Compiling release mode

Release mode deactivates reloading and just uses the reloaded crate as a normal crate.

Comment out the line containing `crate-type = ["dylib"]` in the `Cargo.toml` in the `state_manipulation` folder. It would be nice if this step was eliminated, but AFAIK the only way to do that would be to build both the `dylib` and the `rlib` every time. Given how relatively rare release builds are, that seems like a waste of compile time.

Then run `cargo build --release`
