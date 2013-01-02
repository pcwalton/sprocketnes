//
// sprocketnes/main.rs
//
// Copyright (c) 2012 Mozilla Foundation
// Author: Patrick Walton
//

use cpu::Cpu;
use mapper::Mapper;
use rom::Rom;

use core::cast::transmute;
use core::libc::size_t;
use core::{libc, os, str};

// Currently io GC's. This is obviously bad. To work around this I am not using it.
pub fn println(s: &str) {
    unsafe {
        libc::write(2, transmute(&s[0]), s.len() as size_t); 
        libc::write(2, transmute(&'\n'), 1);
    }
}

fn main() {
    let args = os::args();
    if args.len() < 2 {
        println("usage: sprocketnes <path-to-rom>");
        return;
    }

    let rom = Rom::from_path(args[1]);
    println("Loaded ROM:");
    println(rom.header.to_str());

    let mapper = Mapper::new(&rom);
    let mut cpu = Cpu::new(mapper);

    // TODO: For testing purposes (nestest.log)...
    // cpu.reset();

    for 1000.times {
        cpu.step();
    }
}

