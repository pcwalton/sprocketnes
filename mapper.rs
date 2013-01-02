//
// sprocketnes/mapper.rs
//
// Copyright (c) 2013 Mozilla Foundation
// Author: Patrick Walton
//

use cpu::Mem;
use rom::Rom;

pub struct Mapper {
    rom: &Rom,

    // FIXME: ~fn is awful here; we need the function region work.
    // FIXME: We should trait-ify this. It's hard because of the region in Rom though. I guess we
    // need region-parameterized &Traits to fix this. Dependent on the function region work.
    // NB: This exposed a nasty Rust bug! ~IMapper segfaults; see "segfault" branch.
    loadb: ~fn(this: &mut Mapper, addr: u16) -> u8,
    storeb: ~fn(this: &mut Mapper, addr: u16, val: u8),
    loadw: ~fn(this: &mut Mapper, addr: u16) -> u16,
    storew: ~fn(this: &mut Mapper, addr: u16, val: u16),
}

trait IMapper {
    fn loadb(self, this: &mut Mapper, addr: u16) -> u8;
    fn storeb(self, this: &mut Mapper, addr: u16, val: u8);
    fn loadw(self, this: &mut Mapper, addr: u16) -> u16;
    fn storew(self, this: &mut Mapper, addr: u16, val: u16);
}

pub impl Mapper {
    static fn new(rom: &a/Rom) -> Mapper/&a {
        match rom.header.mapper() {
            0 => Mapper {
                rom: rom,

                loadb:  |this, addr|      Nrom.loadb(this, addr),
                storeb: |this, addr, val| Nrom.storeb(this, addr, val),
                loadw:  |this, addr|      Nrom.loadw(this, addr),
                storew: |this, addr, val| Nrom.storew(this, addr, val)
            },
            _ => fail ~"unsupported mapper"
        }
    }
}

// Forward onto the interface.
pub impl Mapper : Mem {
    fn loadb(&mut self, addr: u16) -> u8      { (self.loadb)(self, addr) }
    fn storeb(&mut self, addr: u16, val: u8)  { (self.storeb)(self, addr, val) }
    fn loadw(&mut self, addr: u16) -> u16     { (self.loadw)(self, addr) }
    fn storew(&mut self, addr: u16, val: u16) { (self.storew)(self, addr, val) }
}

//
// Mapper 0 (NROM)
//
// See http://wiki.nesdev.com/w/index.php/NROM
//

// TODO: RAM.
pub struct Nrom;

// TODO: Support 32K ROMs (NROM-256).
pub impl Nrom : IMapper {
    fn loadb(self, this: &mut Mapper, addr: u16) -> u8 {
        if addr <= 0x8000 {
            0   // FIXME
        } else {
            // FIXME: Unsafe get for speed?
            this.rom.prg[addr & 0x3fff]
        }
    }
    fn storeb(self, _: &mut Mapper, _: u16, _: u8) {
        // TODO
    }

    fn loadw(self, this: &mut Mapper, addr: u16) -> u16 {
        // FIXME: On x86 use unsafe code to do an unaligned read.
        self.loadb(this, addr) as u16 | (self.loadb(this, addr + 1) as u16 << 8)
    }

    fn storew(self, _: &mut Mapper, _: u16, _: u16) {
        // TODO
    }
}

