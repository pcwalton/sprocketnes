//
// sprocketnes/mapper.rs
//
// Author: Patrick Walton
//

use mem::Mem;
use rom::Rom;

use core::cast::transmute;

pub trait Mapper {
    fn prg_loadb(&mut self, addr: u16) -> u8;
    fn prg_storeb(&mut self, addr: u16, val: u8);
}

impl Mapper {
    static fn with_mapper<R>(rom: *Rom, f: &fn(&Mapper) -> R) -> R {
        unsafe {
            match (*rom).header.mapper() {
                0 => {
                    unsafe {
                        let mut nrom = Nrom { rom: rom };
                        let mut nrom_ptr: &static/Nrom = transmute(&mut nrom);  // FIXME: Wat?
                        f(nrom_ptr as &Mapper)
                    }
                },
                _ => fail!(~"unsupported mapper")
            }
        }
    }
}

//
// Mapper 0 (NROM)
//
// See http://wiki.nesdev.com/w/index.php/NROM
//

// TODO: RAM.
pub struct Nrom {
    rom: *Rom,
}

impl Mapper for Nrom {
    fn prg_loadb(&mut self, addr: u16) -> u8 {
        if addr < 0x8000 {
            0   // FIXME
        } else {
            unsafe {
                // FIXME: Unsafe get for speed?
                if (*self.rom).prg.len() > 16384 {
                    (*self.rom).prg[addr & 0x7fff]
                } else {
                    (*self.rom).prg[addr & 0x3fff]
                }
            }
        }
    }
    fn prg_storeb(&mut self, _: u16, _: u8) {
        // TODO
    }
}

