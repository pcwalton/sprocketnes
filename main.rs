//
// sprocketnes/main.rs
//
// Copyright (c) 2012 Mozilla Foundation
// Author: Patrick Walton
//

use cpu::Cpu;
use gfx::Gfx;
use input::Input;
use input;
use mapper::Mapper;
use mem::MemMap;
use ppu::{Oam, Ppu, Vram};
use rom::Rom;
use util::println;

use core::cast::transmute;
use core::libc::size_t;
use core::task::PlatformThread;
use core::{libc, os, str};
use sdl;

// FIXME: This is wrong; we should DRAIN the event queue.
fn check_input() -> bool {
    match sdl::event::poll_event() {
        sdl::event::KeyUpEvent(*) => false,
        _ => true
    }
}

fn start() {
    let args = os::args();
    if args.len() < 2 {
        println("usage: sprocketnes <path-to-rom>");
        return;
    }

    let rom = Rom::from_path(args[1]);
    println("Loaded ROM:");
    println(rom.header.to_str());

    let gfx = Gfx::new();
    let mut mapper = Mapper::new(&rom);
    let mut ppu = Ppu::new(Vram::new(&rom), Oam::new());
    let mut input = Input::new();
    let mut memmap = MemMap::new(ppu, mapper);
    let mut cpu = Cpu::new(memmap);

    // TODO: Add a flag to not reset for nestest.log 
    cpu.reset();

    loop {
        cpu.step();

        let ppu_result = cpu.mem.ppu.step(cpu.cy);
        if ppu_result.vblank_nmi {
            cpu.nmi();
        }

        if ppu_result.new_frame {
            gfx.blit(cpu.mem.ppu.screen);
            gfx.screen.flip();

            match input.check_input() {
                input::Continue => {}
                input::Quit => break
            }
        }
    }
}

fn main() {
    sdl::start::start(start);
}

