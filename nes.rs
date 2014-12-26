//
// sprocketnes/nes.rs
//
// Author: Patrick Walton
//

#![feature(link_args, macro_rules)]

extern crate libc;
extern crate sdl2;

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

fn main() {
    main::start();
}
