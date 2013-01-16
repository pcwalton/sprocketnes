//
// sprocketnes/ppu.rs
//
// Copyright (c) 2013 Mozilla Foundation
// Author: Patrick Walton
//

use mem::Mem;
use rom::Rom;
use util::debug_assert;

//
// Registers
//

struct Regs {
    ctrl: PpuCtrl,      // PPUCTRL: 0x2000
    mask: PpuMask,      // PPUMASK: 0x2001
    status: PpuStatus,  // PPUSTATUS: 0x2002
    oam_addr: u8,       // OAMADDR: 0x2003
    scroll: PpuScroll,  // PPUSCROLL: 0x2005
    addr: u16,          // PPUADDR: 0x2006
}

//
// PPUCTRL: 0x2000
//

struct PpuCtrl(u8);

enum SpriteSize {
    SpriteSize8x8,
    SpriteSize8x16
}

impl PpuCtrl {
    fn base_nametable_addr(self) -> u16           { 0x2000 + (*self & 0x3) as u16 * 0x400 }
    fn vram_addr_increment(self) -> u16           { if (*self & 0x04) == 0 { 0 } else { 32 } }
    fn sprite_pattern_table_addr(self) -> u16     { if (*self & 0x08) == 0 { 0 } else { 0x1000 } }
    fn background_pattern_table_addr(self) -> u16 { if (*self & 0x10) == 0 { 0 } else { 0x1000 } }
    fn sprite_size(self) -> SpriteSize {
        if (*self & 0x20) == 0 { SpriteSize8x8 } else { SpriteSize8x16 }
    }
    fn vblank_nmi(self) -> bool                   { (*self & 0x80) != 0 }
}

//
// PPUMASK: 0x2001
//

struct PpuMask(u8);

impl PpuMask {
    fn grayscale(self) -> bool               { (*self & 0x01) != 0 }
    fn show_background_on_left(self) -> bool { (*self & 0x02) != 0 }
    fn show_sprites_on_left(self) -> bool    { (*self & 0x04) != 0 }
    fn show_background(self) -> bool         { (*self & 0x08) != 0 }
    fn show_sprites(self) -> bool            { (*self & 0x10) != 0 }
    fn intensify_reds(self) -> bool          { (*self & 0x20) != 0 }
    fn intensify_greens(self) -> bool        { (*self & 0x40) != 0 }
    fn intensity_blues(self) -> bool         { (*self & 0x80) != 0 }
}

//
// PPUSTATUS: 0x2002
//

struct PpuStatus(u8);

impl PpuStatus {
    // TODO: open bus junk in bits [0,5)
    fn set_sprite_overflow(&mut self, val: bool) {
        if val { *self = PpuStatus(**self | 0x20) } else { *self = PpuStatus(**self & !0x20) }
    }
    fn set_sprite_zero_hit(&mut self, val: bool) {
        if val { *self = PpuStatus(**self | 0x40) } else { *self = PpuStatus(**self & !0x40) }
    }
    fn set_in_vblank(&mut self, val: bool) {
        if val { *self = PpuStatus(**self | 0x80) } else { *self = PpuStatus(**self & !0x80) }
    }
}

//
// PPUSCROLL: 0x2005
//

struct PpuScroll {
    x: u8,
    y: u8,
    next: PpuScrollDir
}

enum PpuScrollDir {
    XDir,
    YDir,
}

// PPU VRAM. This implements the same Mem trait that the CPU memory does.

pub struct Vram {
    rom: &Rom,
    nametables: [u8 * 0x1000],  // 4 nametables, 0x400 each
    palette: [u8 * 0x20],
}

pub impl Vram {
    static fn new(rom: &a/Rom) -> Vram/&a {
        Vram {
            rom: rom,
            nametables: [ 0, ..0x1000 ],
            palette: [ 0, ..0x20 ]
        }
    }
}

pub impl Vram : Mem {
    fn loadb(&mut self, addr: u16) -> u8 {
        if addr < 0x2000 {          // Tilesets 0 or 1
            return self.rom.chr[addr]
        }
        if addr < 0x3f00 {          // Name table area
            let addr = addr & 0x0fff;
            return self.nametables[addr]
        }
        if addr < 0x4000 {          // Palette area
            let addr = addr & 0x1f;
            return self.palette[addr]
        }
        fail ~"invalid VRAM read"
    }
    fn storeb(&mut self, addr: u16, val: u8) {
        if addr < 0x2000 {
            return                  // Attempt to write to CHR-ROM; ignore.
        }
        if addr < 0x3f00 {          // Name table area
            let addr = addr & 0x0fff;
            self.nametables[addr] = val;
        } else if addr < 0x4000 {   // Palette area
            let addr = addr & 0x1f;
            self.palette[addr] = val;
        }
    }
}

// Object Attribute Memory (OAM).

pub struct Oam {
    oam: [u8 * 0x100]
}

pub impl Oam {
    static fn new() -> Oam {
        Oam { oam: [ 0, ..0x100 ] }
    }
}

pub impl Oam : Mem {
    fn loadb(&mut self, addr: u16) -> u8     { self.oam[addr] }
    fn storeb(&mut self, addr: u16, val: u8) { self.oam[addr] = val }
}

// The main PPU structure. This structure is separate from the PPU memory just as the CPU is.

pub struct Ppu<VM,OM> {
    regs: Regs,
    vram: VM,
    oam: OM,
}

pub impl<VM:Mem,OM:Mem> Ppu<VM,OM> : Mem {
    // Performs a load of the PPU register at the given CPU address.
    fn loadb(&mut self, addr: u16) -> u8 {
        debug_assert(addr >= 0x2000 && addr < 0x4000, "invalid PPU register");
        match addr & 7 {
            0 => *self.regs.ctrl,
            1 => *self.regs.mask,
            2 => *self.regs.status,
            3 => 0, // OAMADDR is read-only
            4 => fail ~"OAM read unimplemented",
            5 => 0, // PPUSCROLL is read-only
            6 => 0, // PPUADDR is read-only
            7 => self.read_ppudata(),
            _ => fail ~"can't happen"
        }
    }

    // Performs a store to the PPU register at the given CPU address.
    fn storeb(&mut self, addr: u16, val: u8) {
        debug_assert(addr >= 0x2000 && addr < 0x4000, "invalid PPU register");
        match addr & 7 {
            0 => self.regs.ctrl = PpuCtrl(val),
            1 => self.regs.mask = PpuMask(val),
            2 => (),    // PPUSTATUS is read-only
            3 => self.regs.oam_addr = val,
            4 => fail ~"OAM write unimplemented",
            5 => self.update_ppuscroll(val),
            6 => self.update_ppuaddr(val),
            7 => self.write_ppudata(val),
            _ => fail ~"can't happen"
        }
    }
}

pub impl<VM:Mem,OM:Mem> Ppu<VM,OM> {
    static fn new(vram: VM, oam: OM) -> Ppu<VM,OM> {
        Ppu {
            regs: Regs {
                ctrl: PpuCtrl(0),
                mask: PpuMask(0),
                status: PpuStatus(0),
                oam_addr: 0,
                scroll: PpuScroll { x: 0, y: 0, next: XDir },
                addr: 0,
            },
            vram: vram,
            oam: oam
        }
    }

    fn update_ppuscroll(&mut self, val: u8) {
        match self.regs.scroll.next {
            XDir => {
                self.regs.scroll.x = val;
                self.regs.scroll.next = YDir;
            }
            YDir => {
                self.regs.scroll.y = val;
                self.regs.scroll.next = XDir;
            }
        }
    }

    fn update_ppuaddr(&mut self, val: u8) {
        self.regs.addr = (self.regs.addr << 8) | (val as u16);
    }

    fn write_ppudata(&mut self, val: u8) {
        self.vram.storeb(self.regs.addr, val);
        self.regs.addr += self.regs.ctrl.vram_addr_increment();
    }

    fn read_ppudata(&mut self) -> u8 {
        let val = self.vram.loadb(self.regs.addr);
        self.regs.addr += self.regs.ctrl.vram_addr_increment();
        val
    }
}

