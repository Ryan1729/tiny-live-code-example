extern crate open_gl_bindings;
extern crate sdl2;

use open_gl_bindings::gl;

use std::ffi::CStr;
use std::str;

fn find_sdl_gl_driver() -> Option<u32> {
    for (index, item) in sdl2::render::drivers().enumerate() {
        if item.name == "opengl" {
            return Some(index as u32);
        }
    }
    None
}

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("Window", 800, 600)
        .opengl()
        .build()
        .unwrap();
    let canvas = window
        .into_canvas()
        .index(find_sdl_gl_driver().unwrap())
        .build()
        .unwrap();

    let ctx = gl::Gl::load_with(|name| video_subsystem.gl_get_proc_address(name) as *const _);
    canvas.window().gl_set_context_to_current();

    let char_ptr = unsafe { ctx.GetString(gl::VERSION) };
    let c_str: &CStr = unsafe { CStr::from_ptr(std::mem::transmute(char_ptr)) };
    let buf: &[u8] = c_str.to_bytes();
    let str_slice: &str = str::from_utf8(buf).unwrap();
    let version: String = str_slice.to_owned();

    println!("OpenGL version: {}", version);
}
