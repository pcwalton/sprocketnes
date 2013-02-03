//
// sprocketnes/mem.rs
//
// Copyright (c) 2013 Mozilla Foundation
// Author: Patrick Walton
//

use mapper::Mapper;
use ppu::{Oam, Ppu, Vram};
use util::debug_print;

//
// The memory interface
//

/// The basic memory interface
pub trait Mem {
    fn loadb(&mut self, addr: u16) -> u8;
    fn storeb(&mut self, addr: u16, val: u8);
}

pub trait MemUtil {
    fn loadw(&mut self, addr: u16) -> u16;
    fn storew(&mut self, addr: u16, val: u16);
}

impl<M:Mem> M : MemUtil {
    fn loadw(&mut self, addr: u16) -> u16 {
        self.loadb(addr) as u16 | (self.loadb(addr + 1) as u16 << 8)
    }
    fn storew(&mut self, addr: u16, val: u16) {
        self.storeb(addr, (val & 0xff) as u8);
        self.storeb(addr + 1, ((val >> 8) & 0xff) as u8);
    }
}

//
// The NES' paltry 2KB of RAM
//

pub struct Ram {
    data: [u8 * 0x800]
}

pub impl Ram : Mem {
    fn loadb(&mut self, addr: u16) -> u8     { self.data[addr & 0x7ff] }
    fn storeb(&mut self, addr: u16, val: u8) { self.data[addr & 0x7ff] = val }
}

//
// The main CPU memory map
//

pub struct MemMap {
    ram: Ram,
    ppu: Ppu<Vram,Oam>,
    mapper: Mapper,
}

pub impl MemMap {
    static fn new(ppu: Ppu<Vram/&a,Oam>, mapper: Mapper/&a) -> MemMap/&a {
        MemMap {
            ram: Ram { data: [ 0, ..0x800 ] },
            ppu: ppu,
            mapper: mapper
        }
    }
}

pub impl MemMap : Mem {
    fn loadb(&mut self, addr: u16) -> u8 {
        if addr < 0x2000 {
            self.ram.loadb(addr)
        } else if addr < 0x4000 {
            self.ppu.loadb(addr)
        } else if addr < 0x4018 {
            debug_print("I/O regs unimplemented");
            0
        } else {
            (self.mapper.loadb)(&mut self.mapper, addr)
        }
    }
    fn storeb(&mut self, addr: u16, val: u8) {
        if addr < 0x2000 {
            self.ram.storeb(addr, val)
        } else if addr < 0x4000 {
            self.ppu.storeb(addr, val)
        } else if addr < 0x4018 {
            debug_print("I/O regs unimplemented")
        } else {
            (self.mapper.storeb)(&mut self.mapper, addr, val)
        }
    }
}

