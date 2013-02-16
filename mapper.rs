//
// sprocketnes/mapper.rs
//
// Author: Patrick Walton
//

use mem::Mem;
use rom::Rom;

pub struct Mapper {
    rom: *Rom,

    // FIXME: ~fn is awful here; we need the function region work.
    // FIXME: We should trait-ify this. It's hard because of the region in Rom though. I guess we
    // need region-parameterized &Traits to fix this. Dependent on the function region work.
    // NB: This exposed a nasty Rust bug! ~IMapper segfaults; see "segfault" branch.
    loadb: ~fn(this: &mut Mapper, addr: u16) -> u8,
    storeb: ~fn(this: &mut Mapper, addr: u16, val: u8),
}

impl Mapper {
    static fn new(rom: *Rom) -> Mapper {
        unsafe {
            match (*rom).header.mapper() {
                0 => Mapper {
                    rom: rom,
                    loadb:  |this, addr|      Nrom.loadb(this, addr),
                    storeb: |this, addr, val| Nrom.storeb(this, addr, val),
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
pub struct Nrom;

impl Nrom {
    fn loadb(self, this: &mut Mapper, addr: u16) -> u8 {
        if addr < 0x8000 {
            0   // FIXME
        } else {
            unsafe {
                // FIXME: Unsafe get for speed?
                if (*this.rom).prg.len() > 16384 {
                    (*this.rom).prg[addr & 0x7fff]
                } else {
                    (*this.rom).prg[addr & 0x3fff]
                }
            }
        }
    }
    fn storeb(self, _: &mut Mapper, _: u16, _: u8) {
        // TODO
    }
}

