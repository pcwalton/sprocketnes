//
// sprocketnes/util.rs
//
// Author: Patrick Walton
//

use cast::transmute;

use core::libc::{c_int, c_void, size_t, ssize_t, time_t};
use core::libc;
use core::ptr::null;

pub trait Save {
    fn save(&mut self, fd: Fd);
    fn load(&mut self, fd: Fd);
}

//
// Standard library I/O replacements
//
// The standard library I/O currently uses the garbage collector, which I do not want to use.
//

// Blech! This really should go in the standard library!
pub struct Fd(c_int);

impl Drop for Fd {
    fn finalize(&self) {
        unsafe {
            libc::close(**self);
        }
    }
}

impl Fd {
    pub fn read(&self, sz: size_t) -> ~[u8] {
        // FIXME: Don't assume that the entire buffer was read in one chunk.
        unsafe {
            let mut result = vec::from_elem(sz as uint, 0);
            if sz != 0 {
                assert libc::read(**self, transmute(&mut result[0]), sz) as size_t == sz;
            }
            result
        }
    }

    pub fn write(&self, buf: &[u8]) {
        unsafe {
            if buf.len() == 0 {
                return;
            }

            let mut offset = 0;
            while offset < buf.len() {
                let nwritten = libc::write(**self,
                                           transmute(&buf[offset]),
                                           (offset - buf.len()) as size_t);
                if nwritten < 0 {
                    fail!();
                }
                offset += nwritten as uint;
            }
        }
    }
}

// Currently io GC's. This is obviously bad. To work around this I am not using it.
pub fn println(s: &str) {
    unsafe {
        libc::write(2, transmute(&s[0]), s.len() as size_t); 
        libc::write(2, transmute(&'\n'), 1);
    }
}

#[cfg(debug)]
pub fn debug_assert(cond: bool, msg: &str) {
    if !cond {
        println(msg);
    }
}

#[cfg(ndebug)]
pub fn debug_assert(_: bool, _: &str) {}

#[cfg(debug)]
pub fn debug_print(msg: &str) {
    println(msg);
}

#[cfg(ndebug)]
pub fn debug_print(_: &str) {}

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

