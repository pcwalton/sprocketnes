//
// sprocketnes/ppu.rs
//
// Copyright (c) 2013 Mozilla Foundation
// Author: Patrick Walton
//

use cpu::Mem;
use rom::Rom;

struct PpuCtrl(u8);
struct PpuMask(u8);
struct PpuStatus(u8);

struct Regs {
    ctrl: PpuCtrl,      // PPUCTRL: 0x2000
    mask: PpuMask,      // PPUMASK: 0x2001
    status: PpuStatus,  // PPUSTATUS: 0x2002
    oam_addr: u8,       // OAMADDR: 0x2003
}

// PPU memory. This implements the same Mem trait that the CPU memory does.

pub struct PpuMem {
    rom: &Rom,
    nametables: [u8 * 0x1000],  // 4 nametables, 0x400 each
    palette: [u8 * 0x20],
}

impl PpuMem : Mem {
    fn loadb(&mut self, addr: u16) -> u8 {
        if addr < 0x2000 {          // Tilesets 0 or 1
            return self.rom.chr[addr]
        }
        let addr = addr & 0x1fff;
        // TODO
        return 0;
    }
    fn storeb(&mut self, addr: u16, val: u8) {
        // TODO
    }
    fn loadw(&mut self, addr: u16) -> u16 {
        // TODO: Duplicated code, blah. Default implementations would be nice here.
        self.loadb(addr) as u16 | (self.loadb(addr + 1) as u16 << 8)
    }

    fn storew(&mut self, _: u16, _: u16) {
        // TODO
    }
}

// The main PPU structure. This structure is separate from the PPU memory just as the CPU is.

struct Ppu {
    regs: Regs,
}

impl Ppu {
}

