//
// sprocketnes/util.rs
//
// Author: Patrick Walton
//

#![allow(improper_ctypes)]

use libc::{c_int, c_void, time_t, uint8_t, uint16_t, uint32_t, uint64_t};
use std::io::File;
use std::ptr::null;

//
// A tiny custom serialization infrastructure, used for savestates.
//
// TODO: Use the standard library's ToBytes and add a FromBytes -- or don't; this is such a small
// amount of code it barely seems worth it.
//

pub trait Save {
    fn save(&mut self, fd: &mut File);
    fn load(&mut self, fd: &mut File);
}

impl Save for uint8_t {
    fn save(&mut self, fd: &mut File) {
        fd.write(&[ *self ]).unwrap();
    }
    fn load(&mut self, fd: &mut File) {
        let mut buf = [ 0 ];
        fd.read_at_least(buf.len(), &mut buf).unwrap();
        *self = buf[0];
    }
}

impl Save for uint16_t {
    fn save(&mut self, fd: &mut File) {
        fd.write(&[ *self as uint8_t, (*self >> 8) as uint8_t ]).unwrap();
    }
    fn load(&mut self, fd: &mut File) {
        let mut buf = [ 0, 0 ];
        fd.read_at_least(buf.len(), &mut buf).unwrap();
        *self = (buf[0] as uint16_t) | ((buf[1] as uint16_t) << 8);
    }
}

impl Save for uint64_t {
    fn save(&mut self, fd: &mut File) {
        let mut buf = [ 0, ..8 ];
        for i in range(0u, 8) {
            buf[i] = ((*self) >> (i * 8)) as uint8_t;
        }
        fd.write(&mut buf).unwrap();
    }
    fn load(&mut self, fd: &mut File) {
        let mut buf = [ 0, ..8 ];
        fd.read_at_least(buf.len(), &mut buf).unwrap();
        *self = 0;
        for i in range(0u, 8) {
            *self = *self | (buf[i] as uint64_t << (i * 8));
        }
    }
}

impl<'a> Save for &'a mut [uint8_t] {
    fn save(&mut self, fd: &mut File) {
        fd.write(*self).unwrap();
    }
    fn load(&mut self, fd: &mut File) {
        fd.read_at_least(self.len(), *self).unwrap();
    }
}

impl Save for bool {
    fn save(&mut self, fd: &mut File) {
        fd.write(&[ if *self { 0 } else { 1 } ]).unwrap();
    }
    fn load(&mut self, fd: &mut File) {
        let mut val: [uint8_t, ..1] = [ 0 ];
        fd.read_at_least(val.len(), &mut val).unwrap();
        *self = val[0] != 0
    }
}

// A convenience macro to save and load entire structs.
macro_rules! save_struct(
    ($name:ident { $($field:ident),* }) => (
        impl Save for $name {
            fn save(&mut self, fd: &mut File) {
                $(self.$field.save(fd);)*
            }
            fn load(&mut self, fd: &mut File) {
                $(self.$field.load(fd);)*
            }
        }
    )
);

macro_rules! save_enum(
    ($name:ident { $val_0:ident, $val_1:ident }) => (
        impl Save for $name {
            fn save(&mut self, fd: &mut File) {
                let mut val: uint8_t = match *self {
                    $name::$val_0 => 0, $name::$val_1 => 1
                };
                val.save(fd)
            }
            fn load(&mut self, fd: &mut File) {
                let mut val: uint8_t = 0;
                val.load(fd);
                *self = if val == 0 {
                    $name::$val_0
                } else {
                    $name::$val_1
                };
            }
        }
    )
);

//
// Random number generation
//
#[deriving(Copy)]
pub struct Xorshift {
    pub x: uint32_t,
    pub y: uint32_t,
    pub z: uint32_t,
    pub w: uint32_t,
}

impl Xorshift {
    pub fn new() -> Xorshift {
        Xorshift { x: 123456789, y: 362436069, z: 521288629, w: 88675123 }
    }

    pub fn next(&mut self) -> uint32_t {
        let t = self.x ^ (self.x << 11);
        self.x = self.y; self.y = self.z; self.z = self.w;
        self.w = self.w ^ (self.w >> 19) ^ (t ^ (t >> 8));
        self.w
    }
}

//
// Simple assertions
//

#[cfg(debug)]
pub fn debug_assert(cond: bool, msg: &str) {
    if !cond {
        println!("{}", msg);
    }
}

#[cfg(not(debug))]
pub fn debug_assert(_: bool, _: &str) {}

#[cfg(debug)]
pub fn debug_print(msg: &str) {
    println!("{}", msg);
}

#[cfg(not(debug))]
pub fn debug_print(_: &str) {}

//
// Bindings for `gettimeofday(2)`
//

#[allow(non_camel_case_types)]
struct timeval {
    tv_sec: time_t,
    tv_usec: uint32_t,
}

extern {
    fn gettimeofday(tp: *mut timeval, tzp: *const c_void) -> c_int;
}

pub fn current_time_millis() -> uint64_t {
    unsafe {
        let mut tv = timeval { tv_sec: 0, tv_usec: 0 };
        gettimeofday(&mut tv, null());
        (tv.tv_sec as uint64_t) * 1000 + (tv.tv_usec as uint64_t) / 1000
    }
}
