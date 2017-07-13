extern crate open_gl_bindings;
extern crate sdl2;

use open_gl_bindings::gl;

use sdl2::event::Event;

use std::ffi::CStr;
use std::ffi::CString;
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

        ctx.ClearColor(0.0, 0.0, 0.0, 1.0);
        ctx.Enable(gl::BLEND);
        ctx.BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

    }

    let vs = compile_shader(&ctx, VS_SRC, gl::VERTEX_SHADER);

    let fs = compile_shader(&ctx, FS_SRC, gl::FRAGMENT_SHADER);

    let program = link_program(&ctx, vs, fs);

    let mut world_matrix: [f32; 16] = [
        1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
    ];

    let verts: Vec<f32> = get_verts();

    let vertex_buffer = unsafe {
        let mut buffer = 0;

        ctx.GenBuffers(1, &mut buffer as _);
        ctx.BindBuffer(gl::ARRAY_BUFFER, buffer);
        ctx.BufferData(
            gl::ARRAY_BUFFER,
            (verts.len() * std::mem::size_of::<f32>()) as _,
            std::mem::transmute(verts.as_ptr()),
            gl::DYNAMIC_DRAW,
        );

        buffer
    };

    let indices: Vec<gl::types::GLushort> =
        (0..verts.len()).map(|x| x as gl::types::GLushort).collect();

    let index_buffer = unsafe {
        let mut buffer = 0;

        ctx.GenBuffers(1, &mut buffer as _);
        ctx.BindBuffer(gl::ELEMENT_ARRAY_BUFFER, buffer);
        ctx.BufferData(
            gl::ELEMENT_ARRAY_BUFFER,
            (indices.len() * std::mem::size_of::<gl::types::GLushort>()) as _,
            std::mem::transmute(indices.as_ptr()),
            gl::DYNAMIC_DRAW,
        );

        buffer
    };
    unsafe {
        ctx.UseProgram(program);

        let pos_attr = ctx.GetAttribLocation(program, CString::new("position").unwrap().as_ptr());

        ctx.BindBuffer(gl::ARRAY_BUFFER, vertex_buffer);
        ctx.EnableVertexAttribArray(pos_attr as _);
        ctx.VertexAttribPointer(
            pos_attr as _,
            2,
            gl::FLOAT,
            gl::FALSE as _,
            0,
            std::ptr::null(),
        )

    }
    let world_attr =
        unsafe { ctx.GetUniformLocation(program, CString::new("world").unwrap().as_ptr()) };

    let colour_uniform =
        unsafe { ctx.GetUniformLocation(program, CString::new("colour").unwrap().as_ptr()) };

    let mut event_pump = sdl_context.event_pump().unwrap();

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } |
                Event::KeyDown { keycode: Some(sdl2::keyboard::Keycode::Escape), .. } |
                Event::KeyDown { keycode: Some(sdl2::keyboard::Keycode::F10), .. } => {
                    break 'running;
                }
                Event::KeyDown { keycode: Some(sdl2::keyboard::Keycode::Left), .. } => {
                    world_matrix[12] -= 0.1;
                }
                Event::KeyDown { keycode: Some(sdl2::keyboard::Keycode::Right), .. } => {
                    world_matrix[12] += 0.1;
                }
                Event::KeyDown { keycode: Some(sdl2::keyboard::Keycode::Down), .. } => {
                    world_matrix[13] -= 0.1;
                }
                Event::KeyDown { keycode: Some(sdl2::keyboard::Keycode::Up), .. } => {
                    world_matrix[13] += 0.1;
                }
                _ => {}
            }
        }

        unsafe {
            ctx.UniformMatrix4fv(world_attr as _, 1, gl::FALSE, world_matrix.as_ptr() as _);
        }

        draw_frame(&ctx, (verts.len() / 2) as _, colour_uniform);

        if cfg!(debug_assertions) {
            let mut err;
            while {
                err = unsafe { ctx.GetError() };
                err != gl::NO_ERROR
            }
            {
                println!("OpenGL error: {}", err);
            }
            if err != gl::NO_ERROR {
                panic!();
            }

        }

        window.gl_swap_window();

        std::thread::sleep(std::time::Duration::from_millis(8));
    }
}

fn draw_frame(ctx: &gl::Gl, vert_count: gl::types::GLsizei, colour_uniform: gl::types::GLint) {
    unsafe {
        ctx.Clear(
            gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT | gl::STENCIL_BUFFER_BIT,
        );

        ctx.Clear(gl::STENCIL_BUFFER_BIT);

        ctx.Enable(gl::STENCIL_TEST);
        ctx.ColorMask(gl::FALSE, gl::FALSE, gl::FALSE, gl::FALSE);
        ctx.StencilOp(gl::INVERT, gl::INVERT, gl::INVERT);
        ctx.StencilFunc(gl::ALWAYS, 0x1, 0x1);

        ctx.Uniform4f(colour_uniform, 1.0, 1.0, 1.0, 1.0);

        ctx.DrawArrays(gl::TRIANGLE_FAN, 0, vert_count);

        ctx.ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
        ctx.Uniform4f(
            colour_uniform,
            32.0 / 255.0,
            32.0 / 255.0,
            63.0 / 255.0,
            1.0,
        );

        ctx.StencilOp(gl::ZERO, gl::ZERO, gl::ZERO);
        ctx.StencilFunc(gl::EQUAL, 1, 1);
        ctx.DrawArrays(gl::TRIANGLE_FAN, 0, vert_count);
        ctx.Disable(gl::STENCIL_TEST);

        //outline
        ctx.Uniform4f(
            colour_uniform,
            128.0 / 255.0,
            128.0 / 255.0,
            32.0 / 255.0,
            1.0,
        );
        ctx.DrawArrays(gl::LINE_STRIP, 0, vert_count);
    }
}
static VS_SRC: &'static str = "#version 120\n\
    attribute vec2 position;\n\
    uniform mat4 world;\n\
    void main() {\n\
    gl_Position = world * vec4(position, 0.0, 1.0);\n\
    }";

static FS_SRC: &'static str = "#version 120\n\
    uniform vec4 colour;\n\
    void main() {\n\
       gl_FragColor = colour;\n\
    }";

//shader helper functions based on https://gist.github.com/simias/c140d1479ada4d6218c0
fn compile_shader(ctx: &gl::Gl, src: &str, shader_type: gl::types::GLenum) -> gl::types::GLuint {
    let shader;
    unsafe {
        shader = ctx.CreateShader(shader_type);
        // Attempt to compile the shader
        let c_str = CString::new(src.as_bytes()).unwrap();
        ctx.ShaderSource(shader, 1, &c_str.as_ptr(), std::ptr::null());
        ctx.CompileShader(shader);

        // Get the compile status
        let mut status = gl::FALSE as gl::types::GLint;
        ctx.GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);

        // Fail on error
        if status != (gl::TRUE as gl::types::GLint) {
            let mut buffer = [0u8; 512];
            let mut length: i32 = 0;
            ctx.GetShaderInfoLog(
                shader,
                buffer.len() as i32,
                &mut length,
                buffer.as_mut_ptr() as *mut i8,
            );
            panic!(
                "Compiler log (length: {}):\n{}",
                length,
                std::str::from_utf8(
                    std::ffi::CStr::from_ptr(std::mem::transmute(&buffer)).to_bytes(),
                ).unwrap()
            );
        }
    }
    shader
}

fn link_program(ctx: &gl::Gl, vs: gl::types::GLuint, fs: gl::types::GLuint) -> gl::types::GLuint {
    unsafe {
        let program = ctx.CreateProgram();
        ctx.AttachShader(program, vs);
        ctx.AttachShader(program, fs);
        ctx.LinkProgram(program);
        // Get the link status
        let mut status = gl::FALSE as gl::types::GLint;
        ctx.GetProgramiv(program, gl::LINK_STATUS, &mut status);

        // Fail on error
        if status != (gl::TRUE as gl::types::GLint) {
            let mut buffer = [0u8; 512];
            let mut length: i32 = 0;
            ctx.GetProgramInfoLog(
                program,
                buffer.len() as i32,
                &mut length,
                buffer.as_mut_ptr() as *mut i8,
            );
            panic!(
                "Compiler log (length: {}):\n{}",
                length,
                std::str::from_utf8(
                    std::ffi::CStr::from_ptr(std::mem::transmute(&buffer)).to_bytes(),
                ).unwrap()
            );
        }
        program
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
