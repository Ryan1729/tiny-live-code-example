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

use std::f32;
use std::fs::File;

extern crate image;

use image::ImageDecoder;

use std::ffi::CString;
use std::str;

extern crate rusttype;
extern crate unicode_normalization;

use rusttype::{point, vector, Font, FontCollection, PositionedGlyph, Scale};

use common::*;

#[cfg(all(debug_assertions, unix))]
const LIB_PATH: &'static str = "./target/debug/libstate_manipulation.so";
#[cfg(all(debug_assertions, windows))]
const LIB_PATH: &'static str = "./target/debug/state_manipulation.dll";
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

    fn get_vert_vecs(&self) -> Vec<Vec<f32>> {
        unsafe {
            let f = self.library
                .get::<fn() -> Vec<Vec<f32>>>(b"get_vert_vecs\0")
                .unwrap();

            f()
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

    fn get_vert_vecs(&self) -> Vec<Vec<f32>> {
        state_manipulation::get_vert_vecs()
    }
}

macro_rules! opengl_error_check {
    () => {

        if cfg!(debug_assertions) {
            #[allow(unused_unsafe)] {
                if let Some(ref resources) = unsafe { RESOURCES.as_ref() } {
                    let mut err;
                    while {
                        err = unsafe { resources.ctx.GetError() };
                        err != gl::NO_ERROR
                    }
                    {
                        let err_str = match err {
                            gl::INVALID_ENUM => "INVALID_ENUM",
                            gl::INVALID_VALUE => "INVALID_VALUE",
                            gl::INVALID_OPERATION => "INVALID_OPERATION",
                            gl::STACK_OVERFLOW => "STACK_OVERFLOW",
                            gl::STACK_UNDERFLOW => "STACK_UNDERFLOW",
                            gl::OUT_OF_MEMORY => "OUT_OF_MEMORY",
                            _ => "Unknown error type",
                        };
                        println!("OpenGL error: {}({}) on line {} of {}",
                            err_str,
                            err,
                            line!(),
                            file!()
                        );
                    }
                    if err != gl::NO_ERROR {
                        panic!();
                    }
                }
            }
        }
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

type Textures = [gl::types::GLuint; 2];

//this includes the default framebuffer
const FRAMEBUFFER_COUNT: usize = 2;
type FrameBufferHandles = [gl::types::GLuint; FRAMEBUFFER_COUNT];

struct Resources {
    ctx: gl::Gl,
    vertex_buffer: gl::types::GLuint,
    index_buffer: gl::types::GLuint,
    vert_ranges_len: usize,
    vert_ranges: Ranges,
    textures: Textures,
    colour_shader: ColourShader,
    texture_shader: TextureShader,
    frame_buffers: FrameBufferHandles,
    frame_buffer_textures: FrameBufferHandles,
    // frame_buffer_render_buffers: FrameBufferHandles,
    text_resources: TextResources,
    text_render_commands: TextRenderCommands,
}

impl Resources {
    fn new(
        app: &Application,
        ctx: gl::Gl,
        (width, height): (u32, u32),
        cache_dim: (u32, u32),
    ) -> Option<Self> {
        let mut frame_buffers = [0; FRAMEBUFFER_COUNT];
        let mut frame_buffer_textures = [0; FRAMEBUFFER_COUNT];
        let mut frame_buffer_render_buffers = [0; FRAMEBUFFER_COUNT];

        unsafe {
            ctx.Viewport(0, 0, width as _, height as _);

            ctx.Enable(gl::TEXTURE_2D);
            ctx.Enable(gl::BLEND);
            reset_blend_func(&ctx);

            //create framebuffers
            ctx.GenFramebuffers(1, frame_buffers.as_mut_ptr().offset(1));

            ctx.BindFramebuffer(gl::FRAMEBUFFER, frame_buffers[1]);

            ctx.GenTextures(1, frame_buffer_textures.as_mut_ptr().offset(1));

            let texture_handle = frame_buffer_textures[1];

            ctx.BindTexture(gl::TEXTURE_2D, texture_handle);

            ctx.TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA as _,
                width as _,
                height as _,
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                std::ptr::null(),
            );

            ctx.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as _);
            ctx.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as _);
            ctx.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as _);
            ctx.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as _);

            ctx.FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                texture_handle,
                0,
            );

            // unbind
            ctx.BindTexture(gl::TEXTURE_2D, 0);

            ctx.GenRenderbuffers(1, frame_buffer_render_buffers.as_mut_ptr().offset(1));
            ctx.BindRenderbuffer(gl::RENDERBUFFER, frame_buffer_render_buffers[1]);
            ctx.RenderbufferStorage(
                gl::RENDERBUFFER,
                gl::DEPTH24_STENCIL8,
                width as _,
                height as _,
            );

            if cfg!(debug_assertions) {
                for &frame_buffer in frame_buffers.iter() {
                    ctx.BindFramebuffer(gl::FRAMEBUFFER, frame_buffer);

                    let status = ctx.CheckFramebufferStatus(gl::FRAMEBUFFER);
                    debug_assert!(
                        status == gl::FRAMEBUFFER_COMPLETE,
                        "CheckFramebufferStatus returned {0:x}",
                        status
                    );
                }
            }

            clear_all(&ctx, &frame_buffers);

            ctx.BindFramebuffer(gl::FRAMEBUFFER, 0);
        }

        let colour_shader = {
            let vs = compile_shader(&ctx, UNTEXTURED_VS_SRC, gl::VERTEX_SHADER);

            let fs = compile_shader(&ctx, UNTEXTURED_FS_SRC, gl::FRAGMENT_SHADER);

            let program = link_program(&ctx, vs, fs);

            let pos_attr = unsafe {
                ctx.GetAttribLocation(program, CString::new("position").unwrap().as_ptr())
            };
            let matrix_uniform = unsafe {
                ctx.GetUniformLocation(program, CString::new("matrix").unwrap().as_ptr())
            };

            let colour_uniform = unsafe {
                ctx.GetUniformLocation(program, CString::new("colour").unwrap().as_ptr())
            };

            ColourShader {
                program,
                pos_attr,
                matrix_uniform,
                colour_uniform,
            }
        };

        let texture_shader = {
            let vs = compile_shader(&ctx, TEXTURED_VS_SRC, gl::VERTEX_SHADER);

            let fs = compile_shader(&ctx, TEXTURED_FS_SRC, gl::FRAGMENT_SHADER);

            let program = link_program(&ctx, vs, fs);

            let pos_attr = unsafe {
                ctx.GetAttribLocation(program, CString::new("position").unwrap().as_ptr())
            };
            let matrix_uniform = unsafe {
                ctx.GetUniformLocation(program, CString::new("matrix").unwrap().as_ptr())
            };

            let texture_uniforms = unsafe {
                [
                    ctx.GetUniformLocation(program, CString::new("textures[0]").unwrap().as_ptr()),
                    ctx.GetUniformLocation(program, CString::new("textures[1]").unwrap().as_ptr()),
                ]
            };

            let texture_xywh_uniform = unsafe {
                ctx.GetUniformLocation(program, CString::new("texture_xywh").unwrap().as_ptr())
            };

            let texture_index_uniform = unsafe {
                ctx.GetUniformLocation(program, CString::new("texture_index").unwrap().as_ptr())
            };

            let tint_uniform =
                unsafe { ctx.GetUniformLocation(program, CString::new("tint").unwrap().as_ptr()) };

            TextureShader {
                program,
                pos_attr,
                matrix_uniform,
                texture_uniforms,
                texture_xywh_uniform,
                texture_index_uniform,
                tint_uniform,
            }
        };

        unsafe {
            ctx.UseProgram(colour_shader.program);
        }

        let textures = [
            make_texture_from_png(&ctx, "images/cardBack_blue.png"),
            make_texture_from_png(&ctx, "images/cardBack_green.png"),
        ];

        debug_assert!(frame_buffers[1] != 0);
        debug_assert!(frame_buffer_textures[1] != 0);
        debug_assert!(frame_buffer_render_buffers[1] != 0);

        let vertex_buffer = unsafe {
            let mut buffer = 0;

            ctx.GenBuffers(1, &mut buffer as _);

            buffer
        };

        let index_buffer = unsafe {
            let mut buffer = 0;

            ctx.GenBuffers(1, &mut buffer as _);

            buffer
        };

        opengl_error_check!();

        let text_resources = TextResources::new(&ctx, cache_dim);

        let mut result = Resources {
            ctx,
            vert_ranges: [(0, 0); 16],
            vert_ranges_len: 0,
            vertex_buffer,
            index_buffer,
            colour_shader,
            texture_shader,
            textures,
            frame_buffers,
            frame_buffer_textures,
            // frame_buffer_render_buffers,
            text_resources,
            text_render_commands: TextRenderCommands::new(),
        };

        result.set_verts(app.get_vert_vecs());

        Some(result)
    }

    fn set_verts(&mut self, vert_vecs: Vec<Vec<f32>>) {
        let (verts, vert_ranges, vert_ranges_len) = get_verts_and_ranges(vert_vecs);

        unsafe {
            self.ctx.BindBuffer(gl::ARRAY_BUFFER, self.vertex_buffer);
            self.ctx.BufferData(
                gl::ARRAY_BUFFER,
                (verts.len() * std::mem::size_of::<f32>()) as _,
                std::mem::transmute(verts.as_ptr()),
                gl::DYNAMIC_DRAW,
            );
        };

        //TODO (assuming we don't end up manipulating this at all)
        // Does this need to be any longer than the longest single polygon?
        // If not, is the extra GPU memory usage significant?
        let indices: Vec<gl::types::GLushort> =
            (0..verts.len()).map(|x| x as gl::types::GLushort).collect();

        unsafe {
            self.ctx
                .BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.index_buffer);
            self.ctx.BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (indices.len() * std::mem::size_of::<gl::types::GLushort>()) as _,
                std::mem::transmute(indices.as_ptr()),
                gl::DYNAMIC_DRAW,
            );
        };

        self.vert_ranges = vert_ranges;
        self.vert_ranges_len = vert_ranges_len;
    }
}

struct TextRenderCommands(
    Option<TextRenderCommand>,
    Option<TextRenderCommand>,
    Option<TextRenderCommand>,
    Option<TextRenderCommand>,
    Option<TextRenderCommand>,
    Option<TextRenderCommand>,
    Option<TextRenderCommand>,
    Option<TextRenderCommand>,
    Option<TextRenderCommand>,
    Option<TextRenderCommand>,
    Option<TextRenderCommand>,
    Option<TextRenderCommand>,
    Option<TextRenderCommand>,
    Option<TextRenderCommand>,
    Option<TextRenderCommand>,
    Option<TextRenderCommand>,
);

const MAX_TEXT_RENDER_COMMANDS: u8 = 16;

impl TextRenderCommands {
    fn new() -> Self {
        TextRenderCommands(
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
    }

    fn len(&self) -> u8 {
        for i in 0..MAX_TEXT_RENDER_COMMANDS {
            if self[i].is_none() {
                return i;
            }
        }

        MAX_TEXT_RENDER_COMMANDS
    }

    fn push(&mut self, text_render_command: TextRenderCommand) {
        let len = self.len();

        debug_assert!(len <= MAX_TEXT_RENDER_COMMANDS);
        if len <= MAX_TEXT_RENDER_COMMANDS {
            self[len] = Some(text_render_command);
        }
    }
}

impl Index<u8> for TextRenderCommands {
    type Output = Option<TextRenderCommand>;

    fn index<'a>(&'a self, index: u8) -> &'a Option<TextRenderCommand> {
        match index {
            0 => &self.0,
            1 => &self.1,
            2 => &self.2,
            3 => &self.3,
            4 => &self.4,
            5 => &self.5,
            6 => &self.6,
            7 => &self.7,
            8 => &self.8,
            9 => &self.9,
            10 => &self.10,
            11 => &self.11,
            12 => &self.12,
            13 => &self.13,
            14 => &self.14,
            15 => &self.15,
            _ => panic!("invalid TextRenderCommands index"),
        }
    }
}


impl IndexMut<u8> for TextRenderCommands {
    fn index_mut<'a>(&'a mut self, index: u8) -> &'a mut Option<TextRenderCommand> {
        match index {
            0 => &mut self.0,
            1 => &mut self.1,
            2 => &mut self.2,
            3 => &mut self.3,
            4 => &mut self.4,
            5 => &mut self.5,
            6 => &mut self.6,
            7 => &mut self.7,
            8 => &mut self.8,
            9 => &mut self.9,
            10 => &mut self.10,
            11 => &mut self.11,
            12 => &mut self.12,
            13 => &mut self.13,
            14 => &mut self.14,
            15 => &mut self.15,
            _ => panic!("invalid TextRenderCommands index"),
        }
    }
}

struct TextRenderCommand {
    char_tuple: CharTuple,
    char_count: u8,
    coords: (f32, f32),
    width_percentage: f32,
    scale: f32,
    colour: [f32; 4],
    frame_buffer_index: usize,
}


impl TextRenderCommand {
    fn new(
        text: &str,
        coords: (f32, f32),
        width_percentage: f32,
        scale: f32,
        colour: [f32; 4],
        frame_buffer_index: usize,
    ) -> Self {
        debug_assert!(text.len() <= CHAR_TUPLE_CAPACITY);
        let char_count = std::cmp::min(text.len(), CHAR_TUPLE_CAPACITY);

        let mut char_tuple: CharTuple = Default::default();
        let mut chars = text.chars();

        for i in 0..char_count {
            char_tuple[i as u8] = chars.next().unwrap();
        }

        TextRenderCommand {
            char_tuple,
            char_count: char_count as u8,
            coords,
            width_percentage,
            scale,
            colour,
            frame_buffer_index,
        }
    }
    fn get_text(&self) -> String {
        let mut result = String::new();

        for i in 0..self.char_count {
            result.push(self.char_tuple[i]);
        }

        result
    }
}

const CHAR_TUPLE_CAPACITY: usize = 64;

#[derive(Default)]
struct CharTuple(
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
    char,
);

use std::ops::{Index, IndexMut};

impl Index<u8> for CharTuple {
    type Output = char;

    fn index<'a>(&'a self, index: u8) -> &'a char {
        match index {
            0 => &self.0,
            1 => &self.1,
            2 => &self.2,
            3 => &self.3,
            4 => &self.4,
            5 => &self.5,
            6 => &self.6,
            7 => &self.7,
            8 => &self.8,
            9 => &self.9,
            10 => &self.10,
            11 => &self.11,
            12 => &self.12,
            13 => &self.13,
            14 => &self.14,
            15 => &self.15,
            16 => &self.16,
            17 => &self.17,
            18 => &self.18,
            19 => &self.19,
            20 => &self.20,
            21 => &self.21,
            22 => &self.22,
            23 => &self.23,
            24 => &self.24,
            25 => &self.25,
            26 => &self.26,
            27 => &self.27,
            28 => &self.28,
            29 => &self.29,
            30 => &self.30,
            31 => &self.31,
            32 => &self.32,
            33 => &self.33,
            34 => &self.34,
            35 => &self.35,
            36 => &self.36,
            37 => &self.37,
            38 => &self.38,
            39 => &self.39,
            40 => &self.40,
            41 => &self.41,
            42 => &self.42,
            43 => &self.43,
            44 => &self.44,
            45 => &self.45,
            46 => &self.46,
            47 => &self.47,
            48 => &self.48,
            49 => &self.49,
            50 => &self.50,
            51 => &self.51,
            52 => &self.52,
            53 => &self.53,
            54 => &self.54,
            55 => &self.55,
            56 => &self.56,
            57 => &self.57,
            58 => &self.58,
            59 => &self.59,
            60 => &self.60,
            61 => &self.61,
            62 => &self.62,
            63 => &self.63,
            _ => panic!("bad CharTuple index"),
        }
    }
}

impl IndexMut<u8> for CharTuple {
    fn index_mut<'a>(&'a mut self, index: u8) -> &'a mut char {
        match index {
            0 => &mut self.0,
            1 => &mut self.1,
            2 => &mut self.2,
            3 => &mut self.3,
            4 => &mut self.4,
            5 => &mut self.5,
            6 => &mut self.6,
            7 => &mut self.7,
            8 => &mut self.8,
            9 => &mut self.9,
            10 => &mut self.10,
            11 => &mut self.11,
            12 => &mut self.12,
            13 => &mut self.13,
            14 => &mut self.14,
            15 => &mut self.15,
            16 => &mut self.16,
            17 => &mut self.17,
            18 => &mut self.18,
            19 => &mut self.19,
            20 => &mut self.20,
            21 => &mut self.21,
            22 => &mut self.22,
            23 => &mut self.23,
            24 => &mut self.24,
            25 => &mut self.25,
            26 => &mut self.26,
            27 => &mut self.27,
            28 => &mut self.28,
            29 => &mut self.29,
            30 => &mut self.30,
            31 => &mut self.31,
            32 => &mut self.32,
            33 => &mut self.33,
            34 => &mut self.34,
            35 => &mut self.35,
            36 => &mut self.36,
            37 => &mut self.37,
            38 => &mut self.38,
            39 => &mut self.39,
            40 => &mut self.40,
            41 => &mut self.41,
            42 => &mut self.42,
            43 => &mut self.43,
            44 => &mut self.44,
            45 => &mut self.45,
            46 => &mut self.46,
            47 => &mut self.47,
            48 => &mut self.48,
            49 => &mut self.49,
            50 => &mut self.50,
            51 => &mut self.51,
            52 => &mut self.52,
            53 => &mut self.53,
            54 => &mut self.54,
            55 => &mut self.55,
            56 => &mut self.56,
            57 => &mut self.57,
            58 => &mut self.58,
            59 => &mut self.59,
            60 => &mut self.60,
            61 => &mut self.61,
            62 => &mut self.62,
            63 => &mut self.63,
            _ => panic!("bad CharTuple index"),
        }
    }
}

struct TextResources {
    shader: TextShader,
    texture: gl::types::GLuint,
    vertex_buffer: gl::types::GLuint,
}

impl TextResources {
    fn new(ctx: &gl::Gl, (width, height): (u32, u32)) -> Self {
        let shader = {
            let vs = compile_shader(&ctx, FONT_VS_SRC, gl::VERTEX_SHADER);

            let fs = compile_shader(&ctx, FONT_FS_SRC, gl::FRAGMENT_SHADER);

            let program = link_program(&ctx, vs, fs);

            let pos_attr = unsafe {
                ctx.GetAttribLocation(program, CString::new("position").unwrap().as_ptr())
            };

            let tex_attr = unsafe {
                ctx.GetAttribLocation(program, CString::new("texcoord").unwrap().as_ptr())
            };

            let colour_attr =
                unsafe { ctx.GetAttribLocation(program, CString::new("colour").unwrap().as_ptr()) };

            let texture_uniform =
                unsafe { ctx.GetUniformLocation(program, CString::new("tex").unwrap().as_ptr()) };

            debug_assert!(pos_attr != -1);
            debug_assert!(tex_attr != -1);
            debug_assert!(colour_attr != -1);
            debug_assert!(texture_uniform != -1);

            TextShader {
                program,
                pos_attr,
                tex_attr,
                colour_attr,
                texture_uniform,
            }
        };

        let texture = unsafe {
            let mut texture = 0;

            ctx.GenTextures(1, &mut texture as _);
            ctx.BindTexture(gl::TEXTURE_2D, texture);

            ctx.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as _);
            ctx.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as _);
            ctx.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as _);
            ctx.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as _);

            ctx.TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RED as _,
                width as _,
                height as _,
                0,
                gl::RED,
                gl::UNSIGNED_BYTE,
                std::ptr::null() as _,
            );

            texture
        };

        let vertex_buffer = unsafe {
            let mut buffer = 0;

            ctx.GenBuffers(1, &mut buffer as _);

            buffer
        };

        opengl_error_check!();

        TextResources {
            shader,
            texture,
            vertex_buffer,
        }
    }
}

fn main() {
    let mut app = Application::new();

    let font_data = include_bytes!("../fonts/LiberationSerif-Regular.ttf");
    let font = FontCollection::from_bytes(font_data as &[u8])
        .into_font()
        .unwrap();

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let gl_attr = video_subsystem.gl_attr();
    gl_attr.set_stencil_size(1);
    gl_attr.set_context_major_version(2);
    gl_attr.set_context_minor_version(1);

    let canvas: sdl2::render::Canvas<sdl2::video::Window> = video_subsystem
        .window("Window", INITIAL_WINDOW_WIDTH, INITIAL_WINDOW_HEIGHT)
        .opengl()
        .build()
        .unwrap()
        .into_canvas()
        .index(find_sdl_gl_driver().unwrap())
        .build()
        .unwrap();

    let (cache_width, cache_height) = (512, 512);

    let mut text_cache = rusttype::gpu_cache::Cache::new(cache_width, cache_height, 0.1, 0.1);

    unsafe {
        let ctx = gl::Gl::load_with(|name| video_subsystem.gl_get_proc_address(name) as *const _);
        canvas.window().gl_set_context_to_current().unwrap();

        RESOURCES = Resources::new(
            &app,
            ctx,
            canvas.window().drawable_size(),
            (cache_width, cache_height),
        )
    }

    let mut state = app.new_state();

    let mut last_modified = if cfg!(debug_assertions) {
        std::fs::metadata(LIB_PATH).unwrap().modified().unwrap()
    } else {
        //hopefully this is actually compiled out
        std::time::SystemTime::now()
    };


    let platform = Platform {
        draw_poly,
        draw_poly_with_matrix,
        draw_textured_poly,
        draw_textured_poly_with_matrix,
        draw_text,
        draw_layer,
        set_verts,
    };

    let mut events = Vec::new();

    app.update_and_render(&platform, &mut state, &mut events);

    opengl_error_check!();

    if let Some(ref mut resources) = unsafe { RESOURCES.as_mut() } {
        let window = canvas.window();
        //I see a flat 16ms a lot of places. Should I be leaving that slack in here?
        let frame_duration = std::time::Duration::new(0, 16666666);

        let mut event_pump = sdl_context.event_pump().unwrap();

        loop {
            let start = std::time::Instant::now();

            events.clear();

            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. } => events.push(common::Event::Quit),
                    Event::KeyDown {
                        keycode: Some(kc), ..
                    } => events.push(common::Event::KeyDown(unsafe { std::mem::transmute(kc) })),
                    Event::KeyUp {
                        keycode: Some(kc), ..
                    } => events.push(common::Event::KeyUp(unsafe { std::mem::transmute(kc) })),
                    Event::MouseMotion { x, y, .. } => {
                        events.push(common::Event::MouseMove((x, y)))
                    }
                    Event::MouseButtonUp {
                        mouse_btn: sdl2::mouse::MouseButton::Left,
                        ..
                    } => events.push(common::Event::LeftMouseUp),
                    Event::MouseButtonDown {
                        mouse_btn: sdl2::mouse::MouseButton::Left,
                        ..
                    } => events.push(common::Event::LeftMouseDown),
                    Event::MouseButtonUp {
                        mouse_btn: sdl2::mouse::MouseButton::Right,
                        ..
                    } => events.push(common::Event::RightMouseUp),
                    Event::MouseButtonDown {
                        mouse_btn: sdl2::mouse::MouseButton::Right,
                        ..
                    } => events.push(common::Event::RightMouseDown),
                    Event::Window {
                        win_event: sdl2::event::WindowEvent::Resized(w, h),
                        ..
                    } |
                    Event::Window {
                        win_event: sdl2::event::WindowEvent::SizeChanged(w, h),
                        ..
                    } => {
                        events.push(common::Event::WindowSize((w, h)));
                        unsafe {
                            resources.ctx.Viewport(0, 0, w, h);
                        }
                    }
                    _ => {}
                }
            }

            unsafe {
                clear_all(&resources.ctx, &resources.frame_buffers);

                resources.ctx.BindFramebuffer(gl::FRAMEBUFFER, 0);
            }

            if app.update_and_render(&platform, &mut state, &mut events) {
                //quit requested
                break;
            }

            //This is a hack that will hopefully be obsoleted when `Drop` types are
            //allowed in statics. see https://github.com/rust-lang/rust/issues/33156
            //We'd rather just store everything we need to do this in `RESOURCES`
            //and call this from `draw_text`
            {
                for i in 0..resources.text_render_commands.len() {
                    if let Some(ref text_render_command) = resources.text_render_commands[i] {
                        let screen_dim = canvas.window().drawable_size();

                        let text = text_render_command.get_text();

                        render_text(
                            &mut text_cache,
                            &font,
                            screen_dim,
                            &text,
                            text_render_command.coords,
                            text_render_command.width_percentage,
                            text_render_command.scale,
                            text_render_command.colour,
                            text_render_command.frame_buffer_index,
                        );
                    }
                    resources.text_render_commands[i] = None;
                }
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

            opengl_error_check!();

            window.gl_swap_window();

            if let Some(sleep_time) =
                frame_duration.checked_sub(std::time::Instant::now().duration_since(start))
            {
                std::thread::sleep(sleep_time);
            }
        }
    } else {
        println!("Could not open window.");
    }
}

fn get_frame_buffer(resources: &Resources, frame_buffer_index: usize) -> gl::types::GLuint {
    resources.frame_buffers[if frame_buffer_index < FRAMEBUFFER_COUNT {
                                frame_buffer_index
                            } else {
                                0
                            }]
}

// these `draw_` functions should probably batch draw calls to minimize shader switching,
// but I'll be able to provide the same API and change to that later so it can wait
fn draw_poly_with_matrix(world_matrix: [f32; 16], poly_index: usize, frame_buffer_index: usize) {
    if let Some(ref resources) = unsafe { RESOURCES.as_ref() } {
        unsafe {
            resources.ctx.UseProgram(resources.colour_shader.program);

            resources.ctx.UniformMatrix4fv(
                resources.colour_shader.matrix_uniform as _,
                1,
                gl::FALSE,
                world_matrix.as_ptr() as _,
            );
        }
        let frame_buffer = get_frame_buffer(resources, frame_buffer_index);

        let (start, end) = resources.vert_ranges[poly_index];

        draw_verts_with_outline(
            &resources.ctx,
            start as _,
            ((end + 1 - start) / 2) as _,
            resources.vertex_buffer,
            &resources.colour_shader,
            frame_buffer,
        );
    }
}

fn draw_poly(x: f32, y: f32, index: usize, frame_buffer_index: usize) {
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

    draw_poly_with_matrix(world_matrix, index, frame_buffer_index);
}

fn draw_textured_poly(
    x: f32,
    y: f32,
    poly_index: usize,
    texture_spec: TextureSpec,
    frame_buffer_index: usize,
) {
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

    draw_textured_poly_with_matrix(world_matrix, poly_index, texture_spec, frame_buffer_index);
}

fn draw_textured_poly_with_matrix(
    world_matrix: [f32; 16],
    poly_index: usize,
    (texture_x, texture_y, texture_w, texture_h, texture_index, tint_r,
tint_g,
tint_b,
tint_a): TextureSpec,
frame_buffer_index: usize
){
    if let Some(ref resources) = unsafe { RESOURCES.as_ref() } {
        unsafe {
            resources.ctx.UseProgram(resources.texture_shader.program);

            resources.ctx.UniformMatrix4fv(
                resources.texture_shader.matrix_uniform as _,
                1,
                gl::FALSE,
                world_matrix.as_ptr() as _,
            );
        }

        let frame_buffer = get_frame_buffer(resources, frame_buffer_index);

        let (start, end) = resources.vert_ranges[poly_index];

        draw_verts_with_texture(
            &resources.ctx,
            start as _,
            ((end + 1 - start) / 2) as _,
            resources.vertex_buffer,
            &resources.texture_shader,
            &resources.textures,
            texture_x,
            texture_y,
            texture_w,
            texture_h,
            texture_index,
            tint_r,
            tint_g,
            tint_b,
            tint_a,
            frame_buffer,
        );
    }
}

fn draw_layer(frame_buffer_index: usize, alpha: f32) {
    if let Some(ref mut resources) = unsafe { RESOURCES.as_mut() } {
        let frame_buffer = get_frame_buffer(resources, frame_buffer_index);
        let textures = resources.textures;
        let texture_shader = &resources.texture_shader;
        let vertex_buffer = resources.vertex_buffer;
        let ctx = &resources.ctx;

        let (start, end) = resources.vert_ranges[1];
        let vert_count = ((end + 1 - start) / 2) as _;

        if frame_buffer != 0 {
            unsafe {
                ctx.BindFramebuffer(gl::FRAMEBUFFER, 0);
                ctx.BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

                let world_matrix: [f32; 16] = [
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

                ctx.UseProgram(texture_shader.program);

                resources.ctx.UniformMatrix4fv(
                    resources.texture_shader.matrix_uniform as _,
                    1,
                    gl::FALSE,
                    world_matrix.as_ptr() as _,
                );

                ctx.BindBuffer(gl::ARRAY_BUFFER, vertex_buffer);
                ctx.EnableVertexAttribArray(texture_shader.pos_attr as _);
                ctx.VertexAttribPointer(
                    texture_shader.pos_attr as _,
                    2,
                    gl::FLOAT,
                    gl::FALSE as _,
                    0,
                    std::ptr::null().offset(start as isize * std::mem::size_of::<f32>() as isize),
                );

                ctx.ActiveTexture(gl::TEXTURE0);
                ctx.BindTexture(
                    gl::TEXTURE_2D,
                    resources.frame_buffer_textures[frame_buffer_index],
                );
                ctx.Uniform1i(texture_shader.texture_uniforms[0], 0);

                ctx.ActiveTexture(gl::TEXTURE1);
                ctx.BindTexture(gl::TEXTURE_2D, textures[1]);
                ctx.Uniform1i(texture_shader.texture_uniforms[1], 1);

                ctx.Uniform1i(texture_shader.texture_index_uniform, 0);

                //1 - y = (y * -1) + 1 so this flips the y texture coord
                ctx.Uniform4f(texture_shader.texture_xywh_uniform, 0.0, 1.0, 1.0, -1.0);

                ctx.Uniform4f(texture_shader.tint_uniform, 0.0, 0.0, 0.0, alpha - 1.0);

                ctx.DrawArrays(gl::TRIANGLE_FAN, 0, vert_count);

                ctx.BindFramebuffer(gl::FRAMEBUFFER, 0);

                reset_blend_func(ctx);
            }
        }
    }
}

fn set_verts(vert_vecs: Vec<Vec<f32>>) {
    if let Some(ref mut resources) = unsafe { RESOURCES.as_mut() } {
        resources.set_verts(vert_vecs);
    }
}

unsafe fn begin_using_frame_buffer(ctx: &gl::Gl, frame_buffer: gl::types::GLuint) {
    ctx.BindFramebuffer(gl::FRAMEBUFFER, frame_buffer);
}
unsafe fn end_using_frame_buffer(ctx: &gl::Gl) {
    ctx.BindFramebuffer(gl::FRAMEBUFFER, 0);
}

unsafe fn reset_blend_func(ctx: &gl::Gl) {
    ctx.BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
    // ctx.BlendFunc(gl::ONE, gl::ONE_MINUS_SRC_ALPHA);
}

unsafe fn clear_all(ctx: &gl::Gl, frame_buffers: &FrameBufferHandles) {
    let brightness = 25.0 / 255.0;
    for i in 0..frame_buffers.len() {
        let frame_buffer = frame_buffers[i];
        ctx.BindFramebuffer(gl::FRAMEBUFFER, frame_buffer);

        if i == 0 {
            ctx.ClearColor(brightness, brightness, brightness, 1.0);
        } else {
            ctx.ClearColor(0.0, 0.0, 0.0, 0.0);
        }
        ctx.Clear(
            gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT | gl::STENCIL_BUFFER_BIT,
        );
    }
}

fn draw_verts_with_outline(
    ctx: &gl::Gl,
    start: isize,
    vert_count: gl::types::GLsizei,
    vertex_buffer: gl::types::GLuint,
    colour_shader: &ColourShader,
    frame_buffer: gl::types::GLuint,
) {
    unsafe {
        begin_using_frame_buffer(ctx, frame_buffer);

        ctx.UseProgram(colour_shader.program);

        ctx.BindBuffer(gl::ARRAY_BUFFER, vertex_buffer);
        ctx.EnableVertexAttribArray(colour_shader.pos_attr as _);
        ctx.VertexAttribPointer(
            colour_shader.pos_attr as _,
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

        ctx.Uniform4f(colour_shader.colour_uniform, 1.0, 1.0, 1.0, 1.0);

        ctx.DrawArrays(gl::TRIANGLE_FAN, 0, vert_count);

        ctx.ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
        ctx.Uniform4f(
            colour_shader.colour_uniform,
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
            colour_shader.colour_uniform,
            128.0 / 255.0,
            128.0 / 255.0,
            32.0 / 255.0,
            1.0,
        );
        ctx.DrawArrays(gl::LINE_STRIP, 0, vert_count);

        end_using_frame_buffer(ctx);
    }
}

//TODO can we pull a common sub-procedure out of this and draw_verts_with_outline?
fn draw_verts_with_texture(
    ctx: &gl::Gl,
    start: isize,
    vert_count: gl::types::GLsizei,
    vertex_buffer: gl::types::GLuint,
    texture_shader: &TextureShader,
    textures: &Textures,
    texture_x: gl::types::GLfloat,
    texture_y: gl::types::GLfloat,
    texture_w: gl::types::GLfloat,
    texture_h: gl::types::GLfloat,
    texture_index: gl::types::GLint,
    tint_r: gl::types::GLfloat,
    tint_g: gl::types::GLfloat,
    tint_b: gl::types::GLfloat,
    tint_a: gl::types::GLfloat,
    frame_buffer: gl::types::GLuint,
) {
    unsafe {
        begin_using_frame_buffer(ctx, frame_buffer);

        ctx.UseProgram(texture_shader.program);

        ctx.BindBuffer(gl::ARRAY_BUFFER, vertex_buffer);
        ctx.EnableVertexAttribArray(texture_shader.pos_attr as _);
        ctx.VertexAttribPointer(
            texture_shader.pos_attr as _,
            2,
            gl::FLOAT,
            gl::FALSE as _,
            0,
            std::ptr::null().offset(start * std::mem::size_of::<f32>() as isize),
        );

        ctx.ActiveTexture(gl::TEXTURE0);
        ctx.BindTexture(gl::TEXTURE_2D, textures[0]);
        ctx.Uniform1i(texture_shader.texture_uniforms[0], 0);

        ctx.ActiveTexture(gl::TEXTURE1);
        ctx.BindTexture(gl::TEXTURE_2D, textures[1]);
        ctx.Uniform1i(texture_shader.texture_uniforms[1], 1);

        ctx.Uniform1i(texture_shader.texture_index_uniform, texture_index);

        ctx.Uniform4f(
            texture_shader.texture_xywh_uniform,
            texture_x,
            texture_y,
            texture_w,
            texture_h,
        );

        ctx.Uniform4f(texture_shader.tint_uniform, tint_r, tint_g, tint_b, tint_a);

        ctx.DrawArrays(gl::TRIANGLE_FAN, 0, vert_count);

        end_using_frame_buffer(ctx);
    }
}

fn draw_text(
    text: &str,
    (x, y): (f32, f32),
    width_percentage: f32,
    scale: f32,
    colour: [f32; 4],
    frame_buffer_index: usize,
) {
    if let Some(ref mut resources) = unsafe { RESOURCES.as_mut() } {
        resources.text_render_commands.push(TextRenderCommand::new(
            text,
            (x, y),
            width_percentage,
            scale,
            colour,
            frame_buffer_index,
        ));
    }
}
fn render_text(
    text_cache: &mut rusttype::gpu_cache::Cache,
    font: &Font,
    (screen_width, screen_height): (u32, u32),
    text: &str,
    (x, y): (f32, f32),
    width_percentage: f32,
    scale: f32,
    colour: [f32; 4],
    frame_buffer_index: usize,
) {
    if let Some(ref resources) = unsafe { RESOURCES.as_ref() } {
        //map from -1 to 1 space ("NDC") to 0 to 1 space
        let (x01, y01) = ((x + 1.0) / 2.0, 1.0 - (y + 1.0) / 2.0);

        let ctx = &resources.ctx;

        let paragraph_max_width = (width_percentage * screen_width as f32) as u32;

        let font_scale = Scale::uniform(scale);

        let paragraph_coords = {
            let v_metrics = font.v_metrics(font_scale);

            let no_offset_glyphs = layout_paragraph(
                font,
                Scale::uniform(scale),
                paragraph_max_width,
                text,
                (0.0, v_metrics.line_gap),
            );

            let paragraph_width = no_offset_glyphs.iter().fold(0, |acc, g| {
                std::cmp::max(g.pixel_bounding_box().map(|r| r.max.x).unwrap_or(acc), acc)
            });
            let paragraph_height = no_offset_glyphs.iter().fold(0, |acc, g| {
                std::cmp::max(g.pixel_bounding_box().map(|r| r.max.y).unwrap_or(acc), acc)
            });

            (
                x01 * screen_width as f32 - (paragraph_width as f32 / 2.0),
                y01 * screen_height as f32 - (paragraph_height as f32 / 2.0),
            )
        };

        let glyphs = layout_paragraph(
            font,
            font_scale,
            paragraph_max_width,
            text,
            paragraph_coords,
        );
        for glyph in &glyphs {
            text_cache.queue_glyph(0, glyph.clone());
        }

        let text_resources = &resources.text_resources;
        unsafe {
            ctx.BindFramebuffer(
                gl::FRAMEBUFFER,
                get_frame_buffer(resources, frame_buffer_index),
            );

            ctx.ActiveTexture(gl::TEXTURE2);
            ctx.BindTexture(gl::TEXTURE_2D, text_resources.texture);

            ctx.PixelStorei(gl::UNPACK_ALIGNMENT, 1);
        }
        text_cache
            .cache_queued(|rect, data| unsafe {
                ctx.TexSubImage2D(
                    gl::TEXTURE_2D,
                    0,
                    rect.min.x as _,
                    rect.min.y as _,
                    rect.width() as _,
                    rect.height() as _,
                    gl::RED,
                    gl::UNSIGNED_BYTE,
                    data.as_ptr() as _,
                );
            })
            .unwrap();

        unsafe {
            //back to default
            ctx.PixelStorei(gl::UNPACK_ALIGNMENT, 4);
        }

        opengl_error_check!();

        #[repr(C)]
        #[derive(Copy, Clone, Debug)]
        struct Vertex {
            position: [f32; 2],
            tex_coords: [f32; 2],
            colour: [f32; 4],
        }

        let origin = point(0.0, 0.0);

        let verts: Vec<_> = glyphs
            .iter()
            .flat_map(|g| {
                if let Ok(Some((uv_rect, screen_rect))) = text_cache.rect_for(0, g) {
                    let gl_rect = rusttype::Rect {
                        min: origin +
                            (vector(
                                screen_rect.min.x as f32 / screen_width as f32 - 0.5,
                                1.0 - screen_rect.min.y as f32 / screen_height as f32 - 0.5,
                            )) * 2.0,
                        max: origin +
                            (vector(
                                screen_rect.max.x as f32 / screen_width as f32 - 0.5,
                                1.0 - screen_rect.max.y as f32 / screen_height as f32 - 0.5,
                            )) * 2.0,
                    };
                    vec![
                        Vertex {
                            position: [gl_rect.min.x, gl_rect.max.y],
                            tex_coords: [uv_rect.min.x, uv_rect.max.y],
                            colour,
                        },
                        Vertex {
                            position: [gl_rect.min.x, gl_rect.min.y],
                            tex_coords: [uv_rect.min.x, uv_rect.min.y],
                            colour,
                        },
                        Vertex {
                            position: [gl_rect.max.x, gl_rect.min.y],
                            tex_coords: [uv_rect.max.x, uv_rect.min.y],
                            colour,
                        },
                        Vertex {
                            position: [gl_rect.max.x, gl_rect.min.y],
                            tex_coords: [uv_rect.max.x, uv_rect.min.y],
                            colour,
                        },
                        Vertex {
                            position: [gl_rect.max.x, gl_rect.max.y],
                            tex_coords: [uv_rect.max.x, uv_rect.max.y],
                            colour,
                        },
                        Vertex {
                            position: [gl_rect.min.x, gl_rect.max.y],
                            tex_coords: [uv_rect.min.x, uv_rect.max.y],
                            colour,
                        },
                    ]
                } else {
                    Vec::new()
                }
            })
            .collect();

        let vert_count = verts.len() as gl::types::GLint;

        unsafe {
            let shader = &text_resources.shader;
            ctx.UseProgram(shader.program);

            ctx.BindBuffer(gl::ARRAY_BUFFER, text_resources.vertex_buffer);

            ctx.BufferData(
                gl::ARRAY_BUFFER,
                (vert_count * std::mem::size_of::<Vertex>() as i32) as _,
                std::mem::transmute(verts.as_ptr()),
                gl::DYNAMIC_DRAW,
            );
            ctx.EnableVertexAttribArray(shader.pos_attr as _);
            ctx.VertexAttribPointer(
                shader.pos_attr as _,
                2,
                gl::FLOAT,
                gl::FALSE as _,
                std::mem::size_of::<Vertex>() as _,
                std::ptr::null(),
            );

            opengl_error_check!();
            ctx.EnableVertexAttribArray(shader.tex_attr as _);
            ctx.VertexAttribPointer(
                shader.tex_attr as _,
                2,
                gl::FLOAT,
                gl::FALSE as _,
                std::mem::size_of::<Vertex>() as _,
                std::ptr::null().offset(std::mem::size_of::<[f32; 2]>() as isize),
            );

            opengl_error_check!();
            ctx.EnableVertexAttribArray(shader.colour_attr as _);
            ctx.VertexAttribPointer(
                shader.colour_attr as _,
                4,
                gl::FLOAT,
                gl::FALSE as _,
                std::mem::size_of::<Vertex>() as _,
                std::ptr::null().offset(2 * std::mem::size_of::<[f32; 2]>() as isize),
            );
            opengl_error_check!();

            ctx.ActiveTexture(gl::TEXTURE2);
            ctx.BindTexture(gl::TEXTURE_2D, text_resources.texture);
            ctx.Uniform1i(shader.texture_uniform, 2);

            ctx.Clear(gl::STENCIL_BUFFER_BIT);

            ctx.DrawArrays(gl::TRIANGLES, 0, vert_count);
            ctx.Disable(gl::STENCIL_TEST);

            ctx.BindTexture(gl::TEXTURE_2D, 0);
            ctx.BindFramebuffer(gl::FRAMEBUFFER, 0);
        }
    }
}



struct ColourShader {
    program: gl::types::GLuint,
    pos_attr: gl::types::GLsizei,
    matrix_uniform: gl::types::GLsizei,
    colour_uniform: gl::types::GLsizei,
}


#[cfg_attr(rustfmt, rustfmt_skip)]
static UNTEXTURED_VS_SRC: &'static str = "#version 120\n\
attribute vec2 position;\n\
uniform mat4 matrix;\n\
void main() {\n\
    gl_Position = matrix * vec4(position, -1.0, 1.0);\n\
}";

#[cfg_attr(rustfmt, rustfmt_skip)]
static UNTEXTURED_FS_SRC: &'static str = "#version 120\n\
uniform vec4 colour;\n\
void main() {\n\
   gl_FragColor = colour;\n\
}";

struct TextureShader {
    program: gl::types::GLuint,
    pos_attr: gl::types::GLsizei,
    matrix_uniform: gl::types::GLsizei,
    texture_uniforms: [gl::types::GLsizei; 2],
    texture_xywh_uniform: gl::types::GLsizei,
    texture_index_uniform: gl::types::GLsizei,
    tint_uniform: gl::types::GLsizei,
}

#[cfg_attr(rustfmt, rustfmt_skip)]
//calculating the uvs here might be slower than passing them in.
//Then again, maybe this is faster because of better memory badwidth.
//We'll profile if it becomes a problem.
static TEXTURED_VS_SRC: &'static str = "#version 120\n\
attribute vec2 position;\n\
uniform mat4 matrix;\n\
uniform vec4 texture_xywh;\n\
varying vec2 texcoord;\n\
void main() {\n\
    vec2 corner = vec2(clamp(position.x, -0.5, 0.5), position.y * -0.5) + vec2(0.5);
    texcoord = corner * texture_xywh.zw + texture_xywh.xy;
    gl_Position = matrix * vec4(position, -1.0, 1.0);\n\
}";

#[cfg_attr(rustfmt, rustfmt_skip)]
static TEXTURED_FS_SRC: &'static str = "#version 120\n\
uniform sampler2D textures[2];\n\
uniform int texture_index;\n\
uniform vec4 tint;\n\
varying vec2 texcoord;\n\
void main() {\n\
    vec4 tex;
    if (texture_index == 1) {
        tex = texture2D(textures[1], texcoord);\n\
    } else {
        tex = texture2D(textures[0], texcoord);\n\
    }

    gl_FragColor = tex + tint * tex.a;
}";

struct TextShader {
    program: gl::types::GLuint,
    pos_attr: gl::types::GLsizei,
    tex_attr: gl::types::GLsizei,
    colour_attr: gl::types::GLsizei,
    texture_uniform: gl::types::GLsizei,
}

#[cfg_attr(rustfmt, rustfmt_skip)]
static FONT_VS_SRC: &'static str = "#version 120\n\
attribute vec2 position;\n\
attribute vec2 texcoord;\n\
attribute vec4 colour;\n\
varying vec2 v_texcoord;\n\
varying vec4 v_colour;\n\
void main() {\n\
    gl_Position = vec4(position, 0.0, 1.0);\n\
    v_texcoord = texcoord;\n\
    v_colour = colour;\n\
}";

#[cfg_attr(rustfmt, rustfmt_skip)]
static FONT_FS_SRC: &'static str = "#version 120\n\
uniform sampler2D tex;\n\
varying vec2 v_texcoord;\n\
varying vec4 v_colour;\n\
void main() {\n\
    gl_FragColor = v_colour * vec4(1.0, 1.0, 1.0, texture2D(tex, v_texcoord).r);\n\
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

fn make_texture_from_png(ctx: &gl::Gl, filename: &str) -> gl::types::GLuint {
    let mut texture = 0;

    if let Ok(image) = File::open(filename) {
        let mut decoder = image::png::PNGDecoder::new(image);
        match (
            decoder.dimensions(),
            decoder.colortype(),
            decoder.read_image(),
        ) {
            (Ok((width, height)), Ok(colortype), Ok(pixels)) => {
                let (external_format, data_type) = match colortype {
                    image::ColorType::RGB(8) => (gl::RGB, gl::UNSIGNED_BYTE),
                    image::ColorType::RGB(16) => (gl::RGB, gl::UNSIGNED_SHORT),
                    image::ColorType::RGBA(8) => (gl::RGBA, gl::UNSIGNED_BYTE),
                    image::ColorType::RGBA(16) => (gl::RGBA, gl::UNSIGNED_SHORT),
                    _ => {
                        //TODO make this case more distinct
                        return 0;
                    }
                };

                unsafe {
                    ctx.GenTextures(1, &mut texture as _);
                    ctx.BindTexture(gl::TEXTURE_2D, texture);

                    ctx.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as _);
                    ctx.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as _);
                    ctx.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as _);
                    ctx.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as _);

                    ctx.TexImage2D(
                        gl::TEXTURE_2D,
                        0,
                        gl::RGBA8 as _,
                        width as _,
                        height as _,
                        0,
                        external_format,
                        data_type,
                        (match pixels {
                            image::DecodingResult::U8(v) => v.as_ptr() as _,
                            image::DecodingResult::U16(v) => v.as_ptr() as _,
                        }),
                    );
                }
            }
            _ => {
                return 0;
            }
        }
    }
    return texture;
}

// based on the rusttype gpu_cache example
fn layout_paragraph<'a>(
    font: &'a Font,
    scale: Scale,
    width: u32,
    text: &str,
    (x, y): (f32, f32),
) -> Vec<PositionedGlyph<'a>> {
    use unicode_normalization::UnicodeNormalization;
    let corner = vector(x, y);
    let newline_point = corner.x as i32 + width as i32;

    let mut result = Vec::new();
    let v_metrics = font.v_metrics(scale);
    let advance_height = v_metrics.ascent - v_metrics.descent + v_metrics.line_gap;
    let mut caret = point(0.0, v_metrics.ascent) + corner;
    let mut last_glyph_id = None;
    for c in text.nfc() {
        if c.is_control() {
            match c {
                '\r' => {
                    caret = point(corner.x, caret.y + advance_height);
                }
                '\n' => {}
                _ => {}
            }
            continue;
        }
        let base_glyph = if let Some(glyph) = font.glyph(c) {
            glyph
        } else {
            continue;
        };
        if let Some(id) = last_glyph_id.take() {
            caret.x += font.pair_kerning(scale, id, base_glyph.id());
        }
        last_glyph_id = Some(base_glyph.id());
        let mut glyph = base_glyph.scaled(scale).positioned(caret);
        if let Some(bb) = glyph.pixel_bounding_box() {
            if bb.max.x > newline_point {
                caret = point(corner.x, caret.y + advance_height);
                glyph = glyph.into_unpositioned().positioned(caret);
                last_glyph_id = None;
            }
        }
        caret.x += glyph.unpositioned().h_metrics().advance_width;
        result.push(glyph);
    }
    result
}
