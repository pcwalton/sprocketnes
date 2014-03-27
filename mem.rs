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

use std::cell::RefCell;
use std::io::File;
use std::rc::Rc;

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

pub struct Ram { val: [u8, ..0x800] }

impl Deref<[u8, ..0x800]> for Ram {
    fn deref<'a>(&'a self) -> &'a [u8, ..0x800] {
        &self.val
    }
}

impl Mem for Ram {
    fn loadb(&mut self, addr: u16) -> u8     { self[addr & 0x7ff] }
    fn storeb(&mut self, addr: u16, val: u8) { self[addr & 0x7ff] = val }
}

impl Save for Ram {
    fn save(&mut self, fd: &mut File) {
        let mut array: &mut [u8] = **self;
        array.save(fd)
    }
    fn load(&mut self, fd: &mut File) {
        let mut array: &mut [u8] = **self;
        array.load(fd)
    }
}

//
// The main CPU memory map
//

pub struct MemMap {
    ram: Ram,
    ppu: Ppu,
    input: Input,
    mapper: Rc<RefCell<~Mapper:Freeze+Send>>,
    apu: Apu,
}

impl MemMap {
    pub fn new(ppu: Ppu,
               input: Input,
               mapper: Rc<RefCell<~Mapper:Freeze+Send>>,
               apu: Apu)
               -> MemMap {
        MemMap {
            ram: Ram{val: [ 0, ..0x800 ]},
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
            mapper.get().prg_loadb(addr)
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
            mapper.get().prg_storeb(addr, val)
        }
    }
}

save_struct!(MemMap { ram, ppu, apu })

