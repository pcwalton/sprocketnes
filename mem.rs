//
// sprocketnes/mem.rs
//
// Author: Patrick Walton
//

use apu::Apu;
use input::Input;
use mapper::Mapper;
use ppu::Ppu;
use util::Save;

use libc::{uint8_t, uint16_t};
use std::cell::RefCell;
use std::io::File;
use std::owned::Box;
use std::rc::Rc;

//
// The memory interface
//

/// The basic memory interface
pub trait Mem {
    fn loadb(&mut self, addr: uint16_t) -> uint8_t;
    fn storeb(&mut self, addr: uint16_t, val: uint8_t);
}

pub trait MemUtil {
    fn loadw(&mut self, addr: uint16_t) -> uint16_t;
    fn storew(&mut self, addr: uint16_t, val: uint16_t);
    fn loadw_zp(&mut self, addr: uint8_t) -> uint16_t;
}

impl<M:Mem> MemUtil for M {
    fn loadw(&mut self, addr: uint16_t) -> uint16_t {
        self.loadb(addr) as uint16_t | (self.loadb(addr + 1) as uint16_t << 8)
    }
    fn storew(&mut self, addr: uint16_t, val: uint16_t) {
        self.storeb(addr, (val & 0xff) as uint8_t);
        self.storeb(addr + 1, ((val >> 8) & 0xff) as uint8_t);
    }
    // Like loadw, but has wraparound behavior on the zero page for address 0xff.
    fn loadw_zp(&mut self, addr: uint8_t) -> uint16_t {
        self.loadb(addr as uint16_t) as uint16_t | (self.loadb((addr + 1) as uint16_t) as uint16_t << 8)
    }
}

//
// The NES' paltry 2KB of RAM
//

pub struct Ram { pub val: [uint8_t, ..0x800] }

impl Deref<[uint8_t, ..0x800]> for Ram {
    fn deref<'a>(&'a self) -> &'a [uint8_t, ..0x800] {
        &self.val
    }
}

impl DerefMut<[uint8_t, ..0x800]> for Ram {
    fn deref_mut<'a>(&'a mut self) -> &'a mut [uint8_t, ..0x800] {
        &mut self.val
    }
}

impl Mem for Ram {
    fn loadb(&mut self, addr: uint16_t) -> uint8_t     { self[addr as uint & 0x7ff] }
    fn storeb(&mut self, addr: uint16_t, val: uint8_t) { self[addr as uint & 0x7ff] = val }
}

impl Save for Ram {
    fn save(&mut self, fd: &mut File) {
        (*self).as_mut_slice().save(fd);
    }
    fn load(&mut self, fd: &mut File) {
        (*self).as_mut_slice().load(fd);
    }
}

//
// The main CPU memory map
//

pub struct MemMap {
    pub ram: Ram,
    pub ppu: Ppu,
    pub input: Input,
    pub mapper: Rc<RefCell<Box<Mapper+Send>>>,
    pub apu: Apu,
}

impl MemMap {
    pub fn new(ppu: Ppu,
               input: Input,
               mapper: Rc<RefCell<Box<Mapper+Send>>>,
               apu: Apu)
               -> MemMap {
        MemMap {
            ram: Ram {
                val: [ 0, ..0x800 ]
            },
            ppu: ppu,
            input: input,
            mapper: mapper,
            apu: apu,
        }
    }
}

impl Mem for MemMap {
    fn loadb(&mut self, addr: uint16_t) -> uint8_t {
        if addr < 0x2000 {
            self.ram.loadb(addr)
        } else if addr < 0x4000 {
            self.ppu.loadb(addr)
        } else if addr == 0x4016 {
            self.input.loadb(addr)
        } else if addr <= 0x4018 {
            self.apu.loadb(addr)
        } else if addr < 0x6000 {
            0   // FIXME: I think some mappers use regs in this area?
        } else {
            let mut mapper = self.mapper.borrow_mut();
            mapper.prg_loadb(addr)
        }
    }
    fn storeb(&mut self, addr: uint16_t, val: uint8_t) {
        if addr < 0x2000 {
            self.ram.storeb(addr, val)
        } else if addr < 0x4000 {
            self.ppu.storeb(addr, val)
        } else if addr == 0x4016 {
            self.input.storeb(addr, val)
        } else if addr <= 0x4018 {
            self.apu.storeb(addr, val)
        } else if addr < 0x6000 {
            // Nothing. FIXME: I think some mappers use regs in this area?
        } else {
            let mut mapper = self.mapper.borrow_mut();
            mapper.prg_storeb(addr, val)
        }
    }
}

save_struct!(MemMap { ram, ppu, apu })

