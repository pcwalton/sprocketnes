//
// sprocketnes/util.rs
//
// Author: Patrick Walton
//

use cast::transmute;

use core::libc::{c_int, c_void, size_t, time_t};
use core::libc;
use core::ptr::null;

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

//
// Bindings for `gettimeofday(2)`
//

struct timeval {
    tv_sec: time_t,
    tv_usec: u32,
}

extern {
    fn gettimeofday(tp: *mut timeval, tzp: *c_void) -> c_int;
}

pub fn current_time_millis() -> u64 {
    unsafe {
        let mut tv = timeval { tv_sec: 0, tv_usec: 0 };
        gettimeofday(&mut tv, null());
        (tv.tv_sec as u64) * 1000 + (tv.tv_usec as u64) / 1000
    }
}

