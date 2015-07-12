//
// Author: Patrick Walton
//

use libc::{c_int, c_void, time_t};
use std::fs::File;
use std::io::{self, Read, Write, Result};
use std::ptr::null;

/// Reads until the buffer is filled or the reader signals EOF
pub fn read_to_buf<R: Read>(mut buf: &mut [u8], rd: &mut R) -> io::Result<()> {
    let mut total = 0;
    while total < buf.len() {
        let count = try!(rd.read(&mut buf[total..]));
        total += count;
    }

    Ok(())
}

//
// A tiny custom serialization infrastructure, used for savestates.
//
// TODO: Use the standard library's ToBytes and add a FromBytes -- or don't; this is such a small
// amount of code it barely seems worth it.
//

// TODO: use `serde` (if it's ready) or `rustc-serialize` and `bincode`

pub trait Save {
    fn save(&mut self, fd: &mut File);
    fn load(&mut self, fd: &mut File);
}

impl Save for u8 {
    fn save(&mut self, fd: &mut File) {
        fd.write_all(&[*self]).unwrap();
    }
    fn load(&mut self, fd: &mut File) {
        let mut buf = [ 0 ];
        read_to_buf(&mut buf, fd).unwrap();
        *self = buf[0];
    }
}

impl Save for u16 {
    fn save(&mut self, fd: &mut File) {
        fd.write(&[*self as u8, (*self >> 8) as u8]).unwrap();
    }
    fn load(&mut self, fd: &mut File) {
        let mut buf = [ 0, 0 ];
        read_to_buf(&mut buf, fd).unwrap();
        *self = (buf[0] as u16) | ((buf[1] as u16) << 8);
    }
}

impl Save for u64 {
    fn save(&mut self, fd: &mut File) {
        let mut buf = [0; 8];
        for i in 0..8 {
            buf[i] = ((*self) >> (i * 8)) as u8;
        }
        fd.write_all(&buf).unwrap();
    }
    fn load(&mut self, fd: &mut File) {
        let mut buf = [0; 8];
        read_to_buf(&mut buf, fd).unwrap();
        *self = 0;
        for i in 0..8 {
            *self = *self | (buf[i] as u64) << (i * 8);
        }
    }
}

impl<'a> Save for &'a mut [u8] {
    fn save(&mut self, fd: &mut File) {
        fd.write(*self).unwrap();
    }
    fn load(&mut self, fd: &mut File) {
        read_to_buf(self, fd).unwrap();
    }
}

impl Save for bool {
    fn save(&mut self, fd: &mut File) {
        fd.write(&[ if *self { 0 } else { 1 } ]).unwrap();
    }
    fn load(&mut self, fd: &mut File) {
        let mut val: [u8; 1] = [ 0 ];
        read_to_buf(&mut val, fd).unwrap();
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
                let mut val: u8 = match *self { $name::$val_0 => 0, $name::$val_1 => 1 };
                val.save(fd)
            }
            fn load(&mut self, fd: &mut File) {
                let mut val: u8 = 0;
                val.load(fd);
                *self = if val == 0 { $name::$val_0 } else { $name::$val_1 };
            }
        }
    )
);

//
// Random number generation
//

// TODO use the `rand` crate

#[derive(Copy, Clone)]
pub struct Xorshift {
    pub x: u32,
    pub y: u32,
    pub z: u32,
    pub w: u32,
}

impl Xorshift {
    pub fn new() -> Xorshift {
        Xorshift { x: 123456789, y: 362436069, z: 521288629, w: 88675123 }
    }

    pub fn next(&mut self) -> u32 {
        let t = self.x ^ (self.x << 11);
        self.x = self.y; self.y = self.z; self.z = self.w;
        self.w = self.w ^ (self.w >> 19) ^ (t ^ (t >> 8));
        self.w
    }
}

//
// Bindings for `gettimeofday(2)`
//

// TODO use the `time` crate

#[allow(non_camel_case_types)]
#[repr(C)]
struct timeval {
    tv_sec: time_t,
    tv_usec: u32,
}

extern {
    fn gettimeofday(tp: *mut timeval, tzp: *const c_void) -> c_int;
}

pub fn current_time_millis() -> u64 {
    unsafe {
        let mut tv = timeval { tv_sec: 0, tv_usec: 0 };
        gettimeofday(&mut tv, null());
        (tv.tv_sec as u64) * 1000 + (tv.tv_usec as u64) / 1000
    }
}
