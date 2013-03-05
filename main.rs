//
// sprocketnes/main.rs
//
// Author: Patrick Walton
//

use apu::Apu;
use audio;
use cpu::Cpu;
use gfx::{Gfx, Scale, Scale1x, Scale2x, Scale3x};
use input::Input;
use input;
use mapper::Mapper;
use mapper;
use mem::MemMap;
use ppu::{Oam, Ppu, Vram};
use rom::Rom;
use util::{Fd, ForReading, ForWriting, Save, println};
use util;

use core::os;
use sdl;

#[cfg(debug)]
fn record_fps(last_time: &mut u64, frames: &mut uint) {
    let now = util::current_time_millis();
    if now >= *last_time + 1000 {
        println(fmt!("%u FPS", *frames));
        *frames = 0;
        *last_time = now;
    } else {
        *frames += 1;
    }
}

#[cfg(ndebug)]
fn record_fps(_: &mut u64, _: &mut uint) {}

//
// Argument parsing
//

struct Options {
    rom_path: ~str,
    scale: Scale,
}

fn usage() {
    println("usage: sprocketnes [options] <path-to-rom>");
    println("options:");
    println("    -1 scale by 1x (default)");
    println("    -2 scale by 2x");
    println("    -3 scale by 3x");
}

fn parse_args() -> Option<Options> {
    let mut options = Options { rom_path: ~"", scale: Scale1x };

    let args = os::args();
    for args.eachi |i, arg| {
        if i == 0 {
            loop;
        }

        if str::eq_slice(*arg, "-1") {
            options.scale = Scale1x;
        } else if str::eq_slice(*arg, "-2") {
            options.scale = Scale2x;
        } else if str::eq_slice(*arg, "-3") {
            options.scale = Scale3x;
        } else if arg[0] == ('-' as u8) {
            usage();
            return None;
        } else {
            options.rom_path = copy *arg;
        }
    }

    if options.rom_path.len() == 0 {
        usage();
        return None;
    }

    Some(options)
}

//
// Entry point and main loop
//

fn start() {
    let options = match parse_args() {
        Some(options) => options,
        None => return
    };

    let rom = ~Rom::from_path(options.rom_path);
    println("Loaded ROM:");
    println(rom.header.to_str());

    let mut gfx = Gfx::new(options.scale);
    let audio_buffer = audio::open();

    do Mapper::with_mapper(rom) |mapper| {
        let mut ppu = Ppu::new(Vram::new(mapper), Oam::new());
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
            } else if ppu_result.scanline_irq {
                cpu.irq();
            }

            if ppu_result.new_frame {
                gfx.tick();
                gfx.composite(cpu.mem.ppu.screen);
                gfx.screen.flip();
                record_fps(&mut last_time, &mut frames);

                match cpu.mem.input.check_input() {
                    input::Continue => {}
                    input::Quit => break,
                    input::SaveState => {
                        cpu.save(&Fd::open("state.sav", ForWriting));
                        gfx.status_line.set(~"Saved state");
                    }
                    input::LoadState => {
                        cpu.load(&Fd::open("state.sav", ForReading));
                        gfx.status_line.set(~"Loaded state");
                    }
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

