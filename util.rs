//
// sprocketnes/util.rs
//
// Author: Patrick Walton
//

use cast::transmute;

use core::libc::size_t;
use core::libc;

// Currently io GC's. This is obviously bad. To work around this I am not using it.
pub fn println(s: &str) {
    unsafe {
        libc::write(2, transmute(&s[0]), s.len() as size_t); 
        libc::write(2, transmute(&'\n'), 1);
    }
}

#[cfg(debug)]
pub fn debug_assert(cond: bool, msg: &static/str) {
    if !cond {
        println(msg);
    }
}

#[cfg(ndebug)]
pub fn debug_assert(_: bool, _: &static/str) {}

#[cfg(debug)]
pub fn debug_print(msg: &static/str) {
    println(msg);
}

#[cfg(ndebug)]
pub fn debug_print(_: &static/str) {}


