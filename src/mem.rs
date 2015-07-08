//
// Author: Patrick Walton
//

use apu::Apu;
use input::Input;
use mapper::Mapper;
use ppu::Ppu;
use util::Save;

use std::cell::RefCell;
use std::fs::File;
use std::rc::Rc;
use std::ops::{Deref, DerefMut};

//
// The memory interface
//

/// The basic memory interface
pub trait Mem {
    fn loadb(&mut self, addr: u16) -> u8;
    fn storeb(&mut self, addr: u16, val: u8);

    fn loadw(&mut self, addr: u16) -> u16 {
        self.loadb(addr) as u16 | (self.loadb(addr + 1) as u16) << 8
    }

    fn storew(&mut self, addr: u16, val: u16) {
        self.storeb(addr, (val & 0xff) as u8);
        self.storeb(addr + 1, ((val >> 8) & 0xff) as u8);
    }
    
    /// Like loadw, but has wraparound behavior on the zero page for address 0xff.
    fn loadw_zp(&mut self, addr: u8) -> u16 {
        self.loadb(addr as u16) as u16 | (self.loadb((addr + 1) as u16) as u16) << 8
    }
}

//
// The NES' paltry 2KB of RAM
//

pub struct Ram { pub val: [u8; 0x800] }

impl Deref for Ram {
    type Target = [u8; 0x800];

    fn deref(&self) -> &[u8; 0x800] {
        &self.val
    }
}

impl DerefMut for Ram {
    fn deref_mut(&mut self) -> &mut [u8; 0x800] {
        &mut self.val
    }
}

impl Mem for Ram {
    fn loadb(&mut self, addr: u16) -> u8     { self[addr as usize & 0x7ff] }
    fn storeb(&mut self, addr: u16, val: u8) { self[addr as usize & 0x7ff] = val }
}

impl Save for Ram {
    fn save(&mut self, fd: &mut File) {
        (&mut **self as &mut [u8]).save(fd);
    }
    fn load(&mut self, fd: &mut File) {
        (&mut **self as &mut [u8]).load(fd);
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
                val: [ 0; 0x800 ]
            },
            ppu: ppu,
            input: input,
            mapper: mapper,
            apu: apu,
        }
    }
}

impl Mem for MemMap {
    fn loadb(&mut self, addr: u16) -> u8 {
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
    fn storeb(&mut self, addr: u16, val: u8) {
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

save_struct!(MemMap { ram, ppu, apu });
