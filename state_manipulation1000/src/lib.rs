#[no_mangle]
pub extern "C" fn lib_update_and_render(counter: &mut i64) {
    println!("{}", counter);

    *counter += 1000;
}
