extern crate common;

use common::*;

#[no_mangle]
pub extern "C" fn lib_new_state() -> State {
    State { counter: 0 }
}

#[no_mangle]
pub extern "C" fn lib_update_and_render(_p: &Platform, state: &mut State) {
    println!("{}", state.counter);

    state.counter += 1;
}
