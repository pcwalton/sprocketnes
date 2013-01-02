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
    // FIXME: We may want to trait-ify this. It's hard because of the region in Rom though. Perhaps
    // we need region-parameterized &Traits.
    loadb: ~fn(this: &Mapper, addr: u16) -> u8;
    storeb: ~fn(this: &Mapper, addr: u16, val: u8);
    loadw: ~fn(this: &Mapper, addr: u16) -> u16;
    storew: ~fn(this: &Mapper, addr: u16, val: u16);
    iface: @IMapper,    // FIXME: segfaults if I use ~IMapper, ugh
}

pub trait IMapper {
    fn loadb(&mut self, rom: &Rom, addr: u16) -> u8;
    fn storeb(&mut self, rom: &Rom, addr: u16, val: u8);
    fn loadw(&mut self, rom: &Rom, addr: u16) -> u16;
    fn storew(&mut self, rom: &Rom, addr: u16, val: u16);
}

pub impl Mapper {
    static fn new(rom: &a/Rom) -> Mapper/&a {
        let iface = match rom.header.mapper() {
            0 => @Nrom as @IMapper,
            _ => fail ~"unsupported mapper"
        };
        Mapper { rom: rom, iface: iface }
    }
}

// Forward onto the interface.
pub impl Mapper : Mem {
    fn loadb(&mut self, addr: u16) -> u8      { self.iface.loadb(self.rom, addr) }
    fn storeb(&mut self, addr: u16, val: u8)  { self.iface.storeb(self.rom, addr, val) }
    fn loadw(&mut self, addr: u16) -> u16     { self.iface.loadw(self.rom, addr) }
    fn storew(&mut self, addr: u16, val: u16) { self.iface.storew(self.rom, addr, val) }
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
    fn loadb(&mut self, rom: &Rom, addr: u16) -> u8 {
        if addr <= 0x8000 {
            0   // FIXME
        } else {
            rom.prg[addr & 0x3fff]
        }
    }
    fn storeb(&mut self, _: &Rom, _: u16, _: u8) {
        // TODO
    }

    fn loadw(&mut self, rom: &Rom, addr: u16) -> u16 {
        // FIXME: On x86 use unsafe code to do an unaligned read.
        self.loadb(rom, addr) as u16 | (self.loadb(rom, addr + 1) as u16 << 8)
    }

    fn storew(&mut self, _: &Rom, _: u16, _: u16) {
        // TODO
    }
}

