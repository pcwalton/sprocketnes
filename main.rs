//
// sprocketnes/main.rs
//
// Author: Patrick Walton
//

use apu::Apu;
use audio;
use cpu::Cpu;
use gfx::Gfx;
use input::Input;
use input;
use mapper::Mapper;
use mem::MemMap;
use ppu::{Oam, Ppu, Vram};
use rom::Rom;
use util::println;
use util;

use core::cast::transmute;
use core::libc::size_t;
use core::task::PlatformThread;
use core::{libc, os, str};
use sdl;

#[cfg(debug)]
fn record_fps(last_time: &mut u64, frames: &mut uint) {
    let now = util::current_time_millis();
    if now >= *last_time + 1000 {
        util::println(fmt!("%u FPS", *frames));
        *frames = 0;
        *last_time = now;
    } else {
        *frames += 1;
    }
}

#[cfg(ndebug)]
fn record_fps(_: &mut u64, _: &mut uint) {}

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
    let audio_buffer = audio::open();

    do Mapper::with_mapper(&rom) |mapper| {
        let mut ppu = Ppu::new(Vram::new(&rom), Oam::new());
        let mut input = Input::new();
        let mut apu = Apu::new(audio_buffer);
        let mut memmap = MemMap::new(ppu, input, mapper, apu);
        let mut cpu = Cpu::new(memmap);

        // TODO: Add a flag to not reset for nestest.log
        cpu.reset();

        let mut last_time = util::current_time_millis();
        let mut frames = 0;

        loop {
            cpu.step();

            let ppu_result = cpu.mem.ppu.step(cpu.cy);
            if ppu_result.vblank_nmi {
                cpu.nmi();
            }

            if ppu_result.new_frame {
                gfx.blit(cpu.mem.ppu.screen);

                gfx.screen.flip();

                record_fps(&mut last_time, &mut frames);

                match cpu.mem.input.check_input() {
                    input::Continue => {}
                    input::Quit => break
                }
            }

            cpu.mem.apu.step(cpu.cy);
        }
    }

    audio::close();
}

fn main() {
    sdl::start::start(start);
}

