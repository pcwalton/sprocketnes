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
use util::Save;
use util;

use std::cast;
use std::cell::RefCell;
use std::io::File;
use std::rc::Rc;
use std::str;

#[link(name="SDL")]
#[link(name="SDLmain")]
extern "C" {
}

#[cfg(target_os="macos")]
#[link(name="objc")]
#[link_args="-framework Cocoa"]
extern "C" {
}

#[cfg(debug)]
fn record_fps(last_time: &mut u64, frames: &mut uint) {
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

fn parse_args(argc: i32, argv: **u8) -> Option<Options> {
    let mut options = Options {
        rom_path: "".to_str(),
        scale: Scale1x,
    };

    for i in range(1, argc as int) {
        let arg = unsafe {
            str::raw::from_c_str(cast::transmute(*argv.offset(i)))
        };

        if "-1" == arg {
            options.scale = Scale1x;
        } else if "-2" == arg {
            options.scale = Scale2x;
        } else if "-3" == arg {
            options.scale = Scale3x;
        } else if arg[0] == ('-' as u8) {
            usage();
            return None;
        } else {
            options.rom_path = arg;
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

pub fn start(argc: i32, argv: **u8) {
    let options = match parse_args(argc, argv) {
        Some(options) => options,
        None => return,
    };

    let rom_path: &str = options.rom_path;
    let rom = ~Rom::from_path(&Path::new(rom_path));
    println("Loaded ROM:");
    println(rom.header.to_str());

    let mut gfx = Gfx::new(options.scale);
    let audio_buffer = audio::open();

    let mapper: ~Mapper:Freeze+Send = mapper::create_mapper(rom);
    let mapper = Rc::from_mut(RefCell::new(mapper));
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

        if ppu_result.new_frame {
            gfx.tick();
            gfx.composite(cpu.mem.ppu.screen);
            gfx.screen.flip();
            record_fps(&mut last_time, &mut frames);

            match cpu.mem.input.check_input() {
                input::Continue => {}
                input::Quit => break,
                input::SaveState => {
                    cpu.save(&mut File::create(&Path::new("state.sav")).unwrap());
                    gfx.status_line.set("Saved state".to_str());
                }
                input::LoadState => {
                    cpu.load(&mut File::open(&Path::new("state.sav")).unwrap());
                    gfx.status_line.set("Loaded state".to_str());
                }
            }
        }

        cpu.mem.apu.step(cpu.cy);
    }

    audio::close();
}

