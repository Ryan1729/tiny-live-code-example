extern crate rand;
extern crate common;

use common::*;
use common::Projection::*;

use rand::{StdRng, SeedableRng, Rng};

#[cfg(debug_assertions)]
#[no_mangle]
pub fn new_state() -> State {
    println!("debug on");

    let seed: &[_] = &[42];
    let rng: StdRng = SeedableRng::from_seed(seed);

    make_state(rng)
}
#[cfg(not(debug_assertions))]
#[no_mangle]
pub fn new_state() -> State {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|dur| dur.as_secs())
        .unwrap_or(42);

    println!("{}", timestamp);
    let seed: &[_] = &[timestamp as usize];
    let rng: StdRng = SeedableRng::from_seed(seed);

    make_state(rng)
}

fn make_state(rng: StdRng) -> State {
    let mut state = State {
        rng,
        polys: Vec::new(),
        cam_x: 0.0,
        cam_y: 0.0,
        zoom: 1.0,
    };

    add_random_poly(&mut state);

    state
}


#[no_mangle]
//returns true if quit requested
pub fn update_and_render(p: &Platform, state: &mut State, events: &mut Vec<Event>) -> bool {
    for event in events {
        println!("{:?}", *event);

        match *event {
            Event::Quit |
            Event::KeyDown(Keycode::Escape) |
            Event::KeyDown(Keycode::F10) => {
                return true;
            }
            Event::KeyDown(Keycode::Space) => {
                add_random_poly(state);
            }
            Event::KeyDown(Keycode::R) => {
                state.polys.clear();
                add_random_poly(state);
            }
            Event::KeyDown(Keycode::Up) => {
                state.cam_y += 0.0625;
            }
            Event::KeyDown(Keycode::Down) => {
                state.cam_y -= 0.0625;
            }
            Event::KeyDown(Keycode::Right) => {
                state.cam_x += 0.0625;
            }
            Event::KeyDown(Keycode::Left) => {
                state.cam_x -= 0.0625;
            }
            Event::KeyDown(Keycode::Num0) => {
                state.cam_x = 0.0;
                state.cam_y = 0.0;
                state.zoom = 1.0;
            }
            Event::KeyDown(Keycode::W) => {
                state.zoom *= 1.25;
            }
            Event::KeyDown(Keycode::S) => {
                state.zoom /= 1.25;
            }
            _ => {}
        }
    }

    let aspect_ratio = 800.0 / 600.0;
    let near = 0.5;
    let far = 1024.0;

    let scale = state.zoom * near;
    let top = scale;
    let bottom = -top;
    let right = aspect_ratio * scale;
    let left = -right;

    let projection = get_projection(&ProjectionSpec {
        top,
        bottom,
        left,
        right,
        near,
        far,
        projection: Perspective,
        // projection: Orthographic,
    });

    let camera = [
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
        state.cam_x,
        state.cam_y,
        0.0,
        1.0,
    ];

    let view = mat4x4_mul(&camera, &projection);

    for poly in state.polys.iter() {
        let world_matrix = [
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
            poly.x,
            poly.y,
            0.0,
            1.0,
        ];

        let matrix = mat4x4_mul(&world_matrix, &view);

        // (p.draw_poly_with_matrix)(matrix, poly.index);
        (p.draw_textured_poly_with_matrix)(matrix, poly.index, 0);
    }

    (p.draw_text)("Hello Text rendering!", (0.25, 0.20), 0.5, 96.0);
    (p.draw_text)("Hello Text rendering!", (0.25, 0.0), 1.0, 96.0);


    false
}

fn add_random_poly(state: &mut State) {
    let poly = Polygon {
        x: state.rng.gen_range(-9.0, 10.0) / 10.0,
        y: state.rng.gen_range(-9.0, 10.0) / 10.0,
        index: state.rng.gen_range(0, 6),
        scale: state.rng.gen_range(0.0, 2.0),
    };

    state.polys.push(poly);
}

//These are the verticies of the polygons which can be drawn.
//The index refers to the index of the inner vector within the outer vecton.
#[cfg_attr(rustfmt, rustfmt_skip)]
#[no_mangle]
pub fn get_vert_vecs() -> Vec<Vec<f32>> {
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
        //hexagon
        vec![
        0.002000, -0.439500,
        -0.379618, -0.221482,
        -0.381618, 0.218018,
        -0.002000, 0.439500,
        0.379618, 0.221482,
        0.381618, -0.218018,
        0.002000, -0.439500,
        ],
        //invert 7 point star
        vec![
        -1.037129, 0.000000,
        -0.487625, 0.071884,
        -0.036111, 0.158214,
        0.934421, 0.449993,
        0.470524, 0.146807,
        0.101182, -0.126878,
        -0.646639, -0.810860,
        -0.360230, -0.336421,
        -0.146212, 0.070412,
        0.230783, 1.011126,
        0.178589, 0.459403,
        0.162283, 0.000000,
        0.230783, -1.011126,
        0.038425, -0.491395,
        -0.146212, -0.070412,
        -0.646639, 0.810860,
        -0.247828, 0.426059,
        0.101182, 0.126878,
        0.934421, -0.449993,
        0.408145, -0.276338,
        -0.036111, -0.158214,
        -1.037129, -0.000000,
        ],
        //invers 6 point star
        vec![
        -1.037129, 0.000000,
        -0.583093, -0.055358,
        -0.204743, -0.117901,
        0.517890, -0.299004,
        0.243039, -0.029458,
        -0.000266, 0.236263,
        -0.518564, 0.898180,
        -0.339488, 0.477294,
        -0.204477, 0.118362,
        0.000000, -0.598008,
        0.096008, -0.225207,
        0.204477, 0.118362,
        0.518564, 0.898180,
        0.243605, 0.532652,
        0.000266, 0.236263,
        -0.517890, -0.299004,
        -0.147031, -0.195748,
        0.204743, -0.117901,
        1.037129, 0.000000,
        0.583093, 0.055358,
        0.204743, 0.117901,
        -0.517890, 0.299004,
        -0.243039, 0.029458,
        0.000266, -0.236263,
        0.518564, -0.898180,
        0.339488, -0.477294,
        0.204477, -0.118362,
        -0.000000, 0.598008,
        -0.096008, 0.225207,
        -0.204477, -0.118362,
        -0.518564, -0.898180,
        -0.243605, -0.532652,
        -0.000266, -0.236263,
        0.517890, 0.299004,
        0.147031, 0.195748,
        -0.204743, 0.117901,
        -1.037129, -0.000000,
        -0.583093, -0.055358,
        -0.204743, -0.117901,
        0.517890, -0.299004,
        0.147031, -0.195748,
        -0.204743, -0.117901,
        -1.037129, 0.000000
        ]
    ]
}
