extern crate open_gl_bindings;
extern crate sdl2;

extern crate common;

#[cfg(debug_assertions)]
extern crate libloading;
#[cfg(not(debug_assertions))]
extern crate state_manipulation;

#[cfg(debug_assertions)]
use libloading::Library;

use open_gl_bindings::gl;

use sdl2::event::Event;

use std::ffi::CStr;
use std::ffi::CString;
use std::str;

use common::*;

#[cfg(debug_assertions)]
const LIB_PATH: &'static str = "./target/debug/libstate_manipulation.so";
#[cfg(not(debug_assertions))]
const LIB_PATH: &'static str = "Hopefully compiled out";

#[cfg(debug_assertions)]
struct Application {
    library: Library,
}
#[cfg(not(debug_assertions))]
struct Application {}

#[cfg(debug_assertions)]
impl Application {
    fn new() -> Self {
        let library = Library::new(LIB_PATH).unwrap_or_else(|error| panic!("{}", error));

        Application { library: library }
    }

    fn new_state(&self) -> State {
        unsafe {
            let f = self.library.get::<fn() -> State>(b"new_state\0").unwrap();

            f()
        }
    }

    fn update_and_render(
        &self,
        platform: &Platform,
        state: &mut State,
        events: &Vec<common::Event>,
    ) -> bool {
        unsafe {
            let f = self.library
                .get::<fn(&Platform, &mut State, &Vec<common::Event>) -> bool>(
                    b"update_and_render\0",
                )
                .unwrap();
            f(platform, state, events)
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

    fn update_and_render(
        &self,
        platform: &Platform,
        state: &mut State,
        events: &mut Vec<common::Event>,
    ) -> bool {
        state_manipulation::update_and_render(platform, state, events)
    }
}

fn find_sdl_gl_driver() -> Option<u32> {
    for (index, item) in sdl2::render::drivers().enumerate() {
        if item.name == "opengl" {
            return Some(index as u32);
        }
    }
    None
}

type Ranges = [(u16, u16); 16];

static mut RESOURCES: Option<Resources> = None;

struct Resources {
    ctx: gl::Gl,
    vertex_buffer: gl::types::GLuint,
    pos_attr: gl::types::GLsizei,
    world_attr: gl::types::GLsizei,
    colour_uniform: gl::types::GLint,
    vert_ranges_len: usize,
    vert_ranges: Ranges,
}

impl Resources {
    fn new(ctx: gl::Gl, (width, height): (u32, u32)) -> Option<Self> {
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

        let (verts, vert_ranges, vert_ranges_len) = get_verts_and_ranges(get_vert_vecs());

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

        let pos_attr = unsafe {
            ctx.UseProgram(program);

            ctx.GetAttribLocation(program, CString::new("position").unwrap().as_ptr())
        };
        let world_attr =
            unsafe { ctx.GetUniformLocation(program, CString::new("world").unwrap().as_ptr()) };

        let colour_uniform =
            unsafe { ctx.GetUniformLocation(program, CString::new("colour").unwrap().as_ptr()) };

        Some(Resources {
            ctx,
            vert_ranges,
            vert_ranges_len,
            vertex_buffer,
            pos_attr,
            world_attr,
            colour_uniform,
        })
    }
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

    unsafe {
        let ctx = gl::Gl::load_with(|name| video_subsystem.gl_get_proc_address(name) as *const _);
        canvas.window().gl_set_context_to_current().unwrap();

        RESOURCES = Resources::new(ctx, canvas.window().drawable_size())
    }
    let mut app = Application::new();

    let mut state = app.new_state();

    let mut last_modified = if cfg!(debug_assertions) {
        std::fs::metadata(LIB_PATH).unwrap().modified().unwrap()
    } else {
        //hopefully this is actually compiled out
        std::time::SystemTime::now()
    };


    let platform = Platform { draw_poly };

    let mut events = Vec::new();

    app.update_and_render(&platform, &mut state, &mut events);

    if let Some(ref resources) = unsafe { RESOURCES.as_ref() } {
        let window = canvas.window();

        let mut event_pump = sdl_context.event_pump().unwrap();

        loop {
            events.clear();

            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. } => events.push(common::Event::Quit),
                    Event::KeyDown { keycode: Some(kc), .. } => {
                        events.push(common::Event::KeyDown(unsafe { std::mem::transmute(kc) }))
                    }
                    Event::KeyUp { keycode: Some(kc), .. } => {
                        events.push(common::Event::KeyUp(unsafe { std::mem::transmute(kc) }))
                    }
                    _ => {}
                }
            }

            unsafe {
                resources.ctx.Clear(
                    gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT | gl::STENCIL_BUFFER_BIT,
                );
            }

            if app.update_and_render(&platform, &mut state, &mut events) {
                //quit requested
                break;
            }

            if cfg!(debug_assertions) {
                if let Ok(Ok(modified)) = std::fs::metadata(LIB_PATH).map(|m| m.modified()) {
                    if modified > last_modified {
                        drop(app);
                        app = Application::new();
                        last_modified = modified;
                    }
                }
            }

            if cfg!(debug_assertions) {
                let mut err;
                while {
                    err = unsafe { resources.ctx.GetError() };
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
    } else {
        //TODO make GLOBALs a Result and display the error.
        println!("Could not open window.");
    }

}

fn draw_poly(x: f32, y: f32, index: usize) {

    if let Some(ref resources) = unsafe { RESOURCES.as_ref() } {

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

        world_matrix[12] = x;
        world_matrix[13] = y;

        unsafe {
            resources.ctx.UniformMatrix4fv(
                resources.world_attr as _,
                1,
                gl::FALSE,
                world_matrix.as_ptr() as _,
            );
        }

        let (mut start, mut end) = resources.vert_ranges[index];

        draw_verts_with_outline(
            &resources.ctx,
            start as _,
            ((end + 1 - start) / 2) as _,
            resources.vertex_buffer,
            resources.pos_attr,
            resources.colour_uniform,
        );
    }

}

fn draw_verts_with_outline(
    ctx: &gl::Gl,
    start: isize,
    vert_count: gl::types::GLsizei,
    vertex_buffer: gl::types::GLuint,
    pos_attr: gl::types::GLsizei,
    colour_uniform: gl::types::GLint,
) {
    unsafe {
        ctx.BindBuffer(gl::ARRAY_BUFFER, vertex_buffer);
        ctx.EnableVertexAttribArray(pos_attr as _);
        ctx.VertexAttribPointer(
            pos_attr as _,
            2,
            gl::FLOAT,
            gl::FALSE as _,
            0,
            std::ptr::null().offset(start * std::mem::size_of::<f32>() as isize),
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

fn get_verts_and_ranges(mut vert_vecs: Vec<Vec<f32>>) -> (Vec<f32>, Ranges, usize) {
    let mut verts = Vec::new();
    let mut ranges = [(0, 0); 16];
    let mut used_len = 0;

    let mut start = 0;

    for mut vec in vert_vecs.iter_mut() {
        let end = start + vec.len() - 1;
        println!("{:?}", (start, end));
        ranges[used_len] = (start as u16, end as u16);

        start = end + 1;

        verts.append(&mut vec);
        used_len += 1;
    }

    (verts, ranges, used_len)
}

#[cfg_attr(rustfmt, rustfmt_skip)]
fn get_vert_vecs() -> Vec<Vec<f32>> {
    vec![
        // star heptagon
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
        ],
        // heptagon
        vec![
            0.555765, -0.002168,
            0.344819, -0.435866,
            -0.125783, -0.541348,
            -0.501668, -0.239184,
            -0.499786, 0.243091,
            -0.121556, 0.542313,
            0.348209, 0.433163,
            0.555765, -0.002168,
        ],
        // star hexagon
        vec![
            0.267355, 0.153145,
            0.158858, 0.062321,
            0.357493, -0.060252,
            0.266305, -0.154964,
            0.133401, -0.106415,
            0.126567, -0.339724,
            -0.001050, -0.308109,
            -0.025457, -0.168736,
            -0.230926, -0.279472,
            -0.267355, -0.153145,
            -0.158858, -0.062321,
            -0.357493, 0.060252,
            -0.266305, 0.154964,
            -0.133401, 0.106415,
            -0.126567, 0.339724,
            0.001050, 0.308109,
            0.025457, 0.168736,
            0.230926, 0.279472,
            0.267355, 0.153145,
        ],
        vec![
        0.002000, -0.439500,
        -0.379618, -0.221482,
        -0.381618, 0.218018,
        -0.002000, 0.439500,
        0.379618, 0.221482,
        0.381618, -0.218018,
        0.002000, -0.439500,
        ],
    ]
}
