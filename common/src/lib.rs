//Because the main cratie and state_manipulation both need to know about `State` and
//`Platform` we need this common crate that they both depend on.

#[repr(C)]
pub struct Platform {
    //You can use a struct like this to pass fn pointers to the state_manipulation crate
    //presenting a common API to platform specific functionality. This allows porting to
    //other platform without needing to change state_manipulation at all.
}

#[repr(C)]
pub struct State {
    pub counter: i64,
}
