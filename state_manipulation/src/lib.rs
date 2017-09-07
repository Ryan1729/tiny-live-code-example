extern crate common;

use common::*;

#[cfg(debug_assertions)]
#[no_mangle]
pub extern "C" fn lib_new_state() -> State {
    new_state()
}

pub fn new_state() -> State {
    State { counter: 0 }
}

#[cfg(debug_assertions)]
#[no_mangle]
pub extern "C" fn lib_update_and_render(_p: &Platform, state: &mut State) {
    update_and_render(_p, state)
}

pub fn update_and_render(_p: &Platform, state: &mut State) {
    println!("{}", state.counter);

    state.counter += 1;
}
