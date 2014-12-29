//
// sprocketnes/main.rs
//
// Author: Patrick Walton
//

use apu::Apu;
use audio;
use cpu::Cpu;
use gfx::{Gfx, Scale};
use input::{Input, InputResult};
use mapper::Mapper;
use mapper;
use mem::MemMap;
use ppu::{Oam, Ppu, Vram};
use rom::Rom;
use util::Save;
use util;

use libc::uint64_t;
use std::cell::RefCell;
use std::io::File;
use std::rc::Rc;

#[cfg(debug)]
fn record_fps(last_time: &mut uint64_t, frames: &mut uint) {
    let now = util::current_time_millis();
    if now >= *last_time + 1000 {
        println!("{} FPS", *frames);
        *frames = 0;
        *last_time = now;
    } else {
        *frames += 1;
    }
}

#[cfg(not(debug))]
fn record_fps(_: &mut uint64_t, _: &mut uint) {}

//
// Argument parsing
//

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

    for arg in ::std::os::args().iter() {

        if "-1" == arg.as_slice() {
            options.scale = Scale::Scale1x;
        } else if "-2" == arg.as_slice() {
            options.scale = Scale::Scale2x;
        } else if "-3" == arg.as_slice() {
            options.scale = Scale::Scale3x;
        } else if arg.as_bytes()[0] == b'-' {
            usage();
            return None;
        } else {
            options.rom_path = arg.to_string();
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

pub fn start() {

    let options = match parse_args() {
        Some(options) => options,
        None => return,
    };

    let rom_path = options.rom_path.as_slice();
    let rom = box Rom::from_path(&Path::new(rom_path));
    println!("Loaded ROM:\n{}", rom.header.to_str());

    let mut gfx = Gfx::new(options.scale);
    let audio_buffer = audio::open();

    let mapper: Box<Mapper+Send> = mapper::create_mapper(rom);
    let mapper = Rc::new(RefCell::new(mapper));
    let ppu = Ppu::new(Vram::new(mapper.clone()), Oam::new());
    let input = Input::new();
    let apu = Apu::new(audio_buffer);
    let memmap = MemMap::new(ppu, input, mapper, apu);
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

        cpu.mem.apu.step(cpu.cy);

        if ppu_result.new_frame {
            gfx.tick();
            gfx.composite(&mut *cpu.mem.ppu.screen);
            record_fps(&mut last_time, &mut frames);
            cpu.mem.apu.play_channels();

            match cpu.mem.input.check_input() {
                InputResult::Continue => {}
                InputResult::Quit => break,
                InputResult::SaveState => {
                    cpu.save(&mut File::create(&Path::new("state.sav")).unwrap());
                    gfx.status_line.set("Saved state".to_string());
                }
                InputResult::LoadState => {
                    cpu.load(&mut File::open(&Path::new("state.sav")).unwrap());
                    gfx.status_line.set("Loaded state".to_string());
                }
            }
        }
    }

    audio::close();
}
