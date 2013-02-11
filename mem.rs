//
// sprocketnes/mem.rs
//
// Author: Patrick Walton
//

use input::Input;
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
    fn loadw_zp(&mut self, addr: u8) -> u16;
}

impl<M:Mem> MemUtil for M {
    fn loadw(&mut self, addr: u16) -> u16 {
        self.loadb(addr) as u16 | (self.loadb(addr + 1) as u16 << 8)
    }
    fn storew(&mut self, addr: u16, val: u16) {
        self.storeb(addr, (val & 0xff) as u8);
        self.storeb(addr + 1, ((val >> 8) & 0xff) as u8);
    }
    // Like loadw, but has wraparound behavior on the zero page for address 0xff.
    fn loadw_zp(&mut self, addr: u8) -> u16 {
        self.loadb(addr as u16) as u16 | (self.loadb((addr + 1) as u16) as u16 << 8)
    }
}

//
// The NES' paltry 2KB of RAM
//

pub struct Ram([u8 * 0x800]);
impl Mem for Ram {
    fn loadb(&mut self, addr: u16) -> u8     { self[addr & 0x7ff] }
    fn storeb(&mut self, addr: u16, val: u8) { self[addr & 0x7ff] = val }
}

//
// The main CPU memory map
//

pub struct MemMap {
    ram: Ram,
    ppu: Ppu<Vram,Oam>,
    input: Input,
    mapper: Mapper,
}

impl MemMap {
    static fn new(ppu: Ppu<Vram/&a,Oam>, input: Input, mapper: Mapper/&a) -> MemMap/&a {
        MemMap { ram: Ram([ 0, ..0x800 ]), ppu: ppu, input: input, mapper: mapper }
    }
}

impl Mem for MemMap {
    fn loadb(&mut self, addr: u16) -> u8 {
        if addr < 0x2000 {
            self.ram.loadb(addr)
        } else if addr < 0x4000 {
            self.ppu.loadb(addr)
        } else if addr < 0x4018 {
            self.input.loadb(addr)
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
            self.input.storeb(addr, val)
        } else {
            (self.mapper.storeb)(&mut self.mapper, addr, val)
        }
    }
}

