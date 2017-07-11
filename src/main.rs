mod gl {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

fn main() {
    println!("It least doesn't explode at least!");
}
