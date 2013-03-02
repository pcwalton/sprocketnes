//
// sprocketnes/util.rs
//
// Author: Patrick Walton
//

use core::cast::transmute;
use core::libc::{O_CREAT, O_RDONLY, O_TRUNC, O_WRONLY, SEEK_CUR, c_int, c_void, off_t, size_t};
use core::libc::{ssize_t, time_t};
use core::libc;
use core::ptr::null;
use core::str;

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
    static pub fn open(path: &str, mode: OpenMode) -> Fd {
        unsafe {
            let fd_mode = match mode {
                ForReading => O_RDONLY,
                ForWriting => O_WRONLY | O_CREAT | O_TRUNC
            } as c_int;
            do str::as_c_str(path) |c_path| {
                Fd(libc::open(c_path, fd_mode, 493))
            }
        }
    }

    pub fn read(&self, buf: &mut [u8]) {
        unsafe {
            let mut offset = 0;
            while offset < buf.len() {
                let nread = libc::read(**self,
                                       transmute(&mut buf[offset]),
                                       (buf.len() - offset) as size_t);
                if nread <= 0 {
                    fail!();
                }
                offset += nread as uint;
            }
        }
    }

    pub fn write(&self, buf: &[u8]) {
        unsafe {
            let mut offset = 0;
            while offset < buf.len() {
                let nwritten = libc::write(**self,
                                           transmute(&buf[offset]),
                                           (buf.len() - offset) as size_t);
                if nwritten <= 0 {
                    fail!();
                }
                offset += nwritten as uint;
            }
        }
    }

    pub fn tell(&self) -> off_t { unsafe { libc::lseek(**self, 0, SEEK_CUR as c_int) } }
}

pub enum OpenMode {
    ForReading,
    ForWriting,
}

//
// A tiny custom serialization infrastructure, used for savestates.
//
// TODO: Use the standard library's ToBytes and add a FromBytes -- or don't; this is such a small
// amount of code it barely seems worth it.
//

pub trait Save {
    fn save(&mut self, fd: &Fd);
    fn load(&mut self, fd: &Fd);
}

impl Save for u8 {
    fn save(&mut self, fd: &Fd) { fd.write([ *self ]) }
    fn load(&mut self, fd: &Fd) { let mut buf = [ 0 ]; fd.read(buf); *self = buf[0]; }
}

impl Save for u16 {
    fn save(&mut self, fd: &Fd) { fd.write([ *self as u8, (*self >> 8) as u8 ]) }
    fn load(&mut self, fd: &Fd) {
        let mut buf = [ 0, 0 ];
        fd.read(buf);
        *self = (buf[0] as u16) | ((buf[1] as u16) << 8);
    }
}

impl Save for u64 {
    fn save(&mut self, fd: &Fd) {
        let mut buf = [ 0, ..8 ];
        for uint::range(0, 8) |i| {
            buf[i] = ((*self) >> (i * 8)) as u8;
        }
        fd.write(buf);
    }
    fn load(&mut self, fd: &Fd) {
        let mut buf = [ 0, ..8 ];
        fd.read(buf);
        *self = 0;
        for uint::range(0, 8) |i| {
            *self = *self | (buf[i] as u64 << (i * 8));
        }
    }
}

impl Save for &mut [u8] {
    fn save(&mut self, fd: &Fd) {
        // FIXME: Unsafe due to stupid borrow check bug.
        unsafe {
            let x: &(&[u8]) = transmute(self);
            fd.write(*x);
        }
    }
    fn load(&mut self, fd: &Fd) { fd.read(*self) }
}

impl Save for bool {
    fn save(&mut self, fd: &Fd) { fd.write([ if *self { 0 } else { 1 } ]) }
    fn load(&mut self, fd: &Fd) {
        let mut val: [u8 * 1] = [ 0 ];
        fd.read(val);
        *self = val[0] != 0
    }
}

// A convenience macro to save and load entire structs.
macro_rules! save_struct(
    ($name:ident { $($field:ident),* }) => (
        impl Save for $name {
            fn save(&mut self, fd: &Fd) {
                $(self.$field.save(fd);)*
            }
            fn load(&mut self, fd: &Fd) {
                $(self.$field.load(fd);)*
            }
        }
    )
)

macro_rules! save_enum(
    ($name:ident { $val_0:ident, $val_1:ident }) => (
        impl Save for $name {
            fn save(&mut self, fd: &Fd) {
                let mut val: u8 = match *self { $val_0 => 0, $val_1 => 1 };
                val.save(fd)
            }
            fn load(&mut self, fd: &Fd) {
                let mut val: u8 = 0;
                val.load(fd);
                *self = if val == 0 { $val_0 } else { $val_1 };
            }
        }
    )
)


//
// Basic output
//
// This is reimplemented because the core Rust I/O library currently uses the garbage collector.
//

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

