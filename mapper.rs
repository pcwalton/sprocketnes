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
    fn chr_loadb(&mut self, addr: u16) -> u8;
    fn chr_storeb(&mut self, addr: u16, val: u8);
}

impl Mapper {
    static fn with_mapper<R>(rom: ~Rom, f: &fn(&Mapper) -> R) -> R {
        match rom.header.mapper() {
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

//
// Mapper 0 (NROM)
//
// See http://wiki.nesdev.com/w/index.php/NROM
//

// TODO: RAM.
pub struct Nrom {
    rom: ~Rom,
}

impl Mapper for Nrom {
    fn prg_loadb(&mut self, addr: u16) -> u8 {
        if addr < 0x8000 {
            0
        } else if self.rom.prg.len() > 16384 {
            self.rom.prg[addr & 0x7fff]
        } else {
            self.rom.prg[addr & 0x3fff]
        }
    }
    fn prg_storeb(&mut self, _: u16, _: u8) {}  // Can't store to PRG-ROM.
    fn chr_loadb(&mut self, addr: u16) -> u8 { self.rom.chr[addr] }
    fn chr_storeb(&mut self, _: u16, _: u8) {}  // Can't store to CHR-ROM.
}

