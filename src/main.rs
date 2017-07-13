extern crate open_gl_bindings;
extern crate sdl2;

use open_gl_bindings::gl;

use sdl2::event::Event;

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

    let gl_attr = video_subsystem.gl_attr();
    gl_attr.set_stencil_size(1);
    gl_attr.set_context_major_version(2);
    gl_attr.set_context_minor_version(1);

    let canvas = video_subsystem
        .window("Window", 800, 600)
        .opengl()
        .build()
        .unwrap()
        .into_canvas()
        .index(find_sdl_gl_driver().unwrap())
        .build()
        .unwrap();

    let ctx = gl::Gl::load_with(|name| video_subsystem.gl_get_proc_address(name) as *const _);
    canvas.window().gl_set_context_to_current().unwrap();

    let char_ptr = unsafe { ctx.GetString(gl::VERSION) };
    let c_str: &CStr = unsafe { CStr::from_ptr(std::mem::transmute(char_ptr)) };
    let buf: &[u8] = c_str.to_bytes();
    let str_slice: &str = str::from_utf8(buf).unwrap();
    let version: String = str_slice.to_owned();

    println!("OpenGL version: {}", version);

    let window = canvas.window();

    let (width, height) = window.drawable_size();

    unsafe {
        ctx.Viewport(0, 0, width as _, height as _);

        ctx.MatrixMode(gl::PROJECTION);
        ctx.LoadIdentity();

        ctx.MatrixMode(gl::MODELVIEW);
        ctx.LoadIdentity();
    }

    unsafe {
        ctx.ClearColor(0.0, 0.0, 0.0, 1.0);
        ctx.Clear(
            gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT | gl::STENCIL_BUFFER_BIT,
        );
        ctx.Enable(gl::BLEND);
        ctx.BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

        ctx.Clear(gl::STENCIL_BUFFER_BIT);

        let verts: Vec<f32> = get_verts();

        ctx.EnableClientState(gl::VERTEX_ARRAY);
        ctx.VertexPointer(2, gl::FLOAT, 0, std::mem::transmute(verts.as_ptr()));

        let cnt = (verts.len() / 2) as _;

        ctx.Enable(gl::STENCIL_TEST);

        ctx.ColorMask(gl::FALSE, gl::FALSE, gl::FALSE, gl::FALSE);
        ctx.StencilOp(gl::INVERT, gl::INVERT, gl::INVERT);
        ctx.StencilFunc(gl::ALWAYS, 0x1, 0x1);
        ctx.Color4f(1.0, 1.0, 1.0, 1.0);
        ctx.DrawArrays(gl::TRIANGLE_FAN, 0, cnt);

        ctx.ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
        ctx.Color4f(32.0 / 255.0, 32.0 / 255.0, 63.0 / 255.0, 1.0);

        ctx.StencilOp(gl::ZERO, gl::ZERO, gl::ZERO);
        ctx.StencilFunc(gl::EQUAL, 1, 1);
        ctx.DrawArrays(gl::TRIANGLE_FAN, 0, cnt);
        ctx.Disable(gl::STENCIL_TEST);

        //outline
        ctx.Color4f(128.0 / 255.0, 128.0 / 255.0, 32.0 / 255.0, 1.0);
        ctx.DrawArrays(gl::LINE_STRIP, 0, cnt);

    }

    let mut event_pump = sdl_context.event_pump().unwrap();

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } |
                Event::KeyDown { keycode: Some(sdl2::keyboard::Keycode::Escape), .. } |
                Event::KeyDown { keycode: Some(sdl2::keyboard::Keycode::F10), .. } => {
                    break 'running;
                }
                _ => {}
            }
        }

        window.gl_swap_window();

        std::thread::sleep(std::time::Duration::from_millis(8));
    }
}

#[cfg_attr(rustfmt, rustfmt_skip)]
fn get_verts() -> Vec<f32> {
    vec![
        -0.012640, 0.255336,
        0.152259, 0.386185,
        0.223982, 0.275978,
        0.191749, 0.169082,
        0.396864, 0.121742,
        0.355419, -0.003047,
        0.251747, -0.044495,
        0.342622, -0.234376,
        0.219218, -0.279777,
        0.122174, -0.224565,
        0.030379, -0.414003,
        -0.082058, -0.345830,
        -0.099398, -0.235534,
        -0.304740, -0.281878,
        -0.321543, -0.151465,
        -0.246122, -0.069141,
        -0.410383, 0.062507,
        -0.318899, 0.156955,
        -0.207511, 0.149317,
        -0.207000, 0.359823,
        -0.076118, 0.347186,
        -0.012640, 0.255336,
    ]
}
