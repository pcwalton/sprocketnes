//
// sprocketnes/nes.rs
//
// Author: Patrick Walton
//

#![feature(link_args, macro_rules)]
#![no_main]

extern crate libc;
extern crate native;
extern crate sdl2;

use libc::{int32_t, uint8_t};

// NB: This must be first to pick up the macro definitions. What a botch.
#[macro_escape]
pub mod util;

pub mod apu;
pub mod audio;
#[macro_escape]
pub mod cpu;
pub mod disasm;
pub mod gfx;
pub mod input;
pub mod main;
pub mod mapper;
pub mod mem;
pub mod ppu;
pub mod rom;

// C library support
pub mod speex;

#[no_mangle]
pub extern "C" fn main(argc: int32_t, argv: *const *const uint8_t) -> int32_t {
    native::start(argc as int, argv, proc() main::start(argc, argv)) as int32_t
}

