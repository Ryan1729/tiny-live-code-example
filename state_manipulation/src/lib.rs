extern crate rand;
extern crate common;

use common::*;

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

fn make_state(mut rng: StdRng) -> State {
    let mut state = State {
        rng,
        polys: Vec::new(),
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
            _ => {}
        }
    }

    for &(x, y, poly) in state.polys.iter() {
        (p.draw_poly)(x, y, poly);
    }

    false
}

fn add_random_poly(state: &mut State) {
    state.polys.push((
        state.rng.gen_range(-9.0, 10.0) as f32 / 10.0,
        state.rng.gen_range(-9.0, 10.0) as f32 / 10.0,
        state.rng.gen_range(0, 4),
    ));
}
