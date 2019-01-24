//
// Author: Patrick Walton
//

extern crate nes;

use nes::gfx::Scale;
use nes::rom::Rom;

use std::env;
use std::fs::File;
use std::path::Path;

struct Options {
    rom_path: String,
    scale: Scale,
}

fn usage() {
    println!("usage: sprocketnes [options] <path-to-rom>");
    println!("options:");
    println!("    -1 scale by 1x (default)");
    println!("    -2 scale by 2x");
    println!("    -3 scale by 3x");
}

fn parse_args() -> Option<Options> {
    let mut options = Options {
        rom_path: String::new(),
        scale: Scale::Scale1x,
    };

    for arg in env::args().skip(1) {
        match &*arg {
            "-1" => {
                options.scale = Scale::Scale1x;
            }
            "-2" => {
                options.scale = Scale::Scale2x;
            }
            "-3" => {
                options.scale = Scale::Scale3x;
            }
            _ if arg.starts_with('-') => {
                usage();
                return None;
            }
            _ => {
                options.rom_path = arg;
            }
        }
    }

    if options.rom_path.len() == 0 {
        usage();
        return None;
    }

    Some(options)
}

fn main() {
    let options = match parse_args() {
        Some(options) => options,
        None => return,
    };

    let rom_path = &options.rom_path;
    let rom = Rom::load(&mut File::open(&Path::new(rom_path)).unwrap()).unwrap();

    nes::start_emulator(rom, options.scale);
}
