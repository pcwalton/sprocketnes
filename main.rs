//
// sprocketnes/main.rs
//
// Copyright (c) 2012 Mozilla Foundation
// Author: Patrick Walton
//

use cpu::{Cpu, SimpleMem};
use rom::Rom;

use core::cast::transmute;
use core::libc::size_t;
use core::{libc, os, str};

// Currently io GC's. This is obviously bad. To work around this I am not using it.
fn println(s: &str) {
    unsafe {
        libc::write(2, transmute(&s[0]), s.len() as size_t); 
        libc::write(2, transmute(&'\n'), 1);
    }
}

fn main() {
    let args = os::args();
    if args.len() < 1 {
        println("usage: sprocketnes <path-to-rom>");
        return;
    }

    let rom = Rom::from_path(args[1]);
    debug!("io header is %?", rom.header);

    let mut cpu = Cpu::new(SimpleMem { data: [ 0, ..65536 ] });
    cpu.step();
}

