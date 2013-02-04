//
// sprocketnes/ppu.rs
//
// Copyright (c) 2013 Mozilla Foundation
// Author: Patrick Walton
//

use mem::Mem;
use rom::Rom;
use util::{debug_assert, debug_print};

use core::uint::range;

//
// Constants
//

pub const SCREEN_WIDTH: uint = 256;
pub const SCREEN_HEIGHT: uint = 240;
pub const CYCLES_PER_SCANLINE: u64 = 124;   // 29781 cycles per frame, 240 scanlines
pub const VBLANK_SCANLINE: uint = 241;
pub const LAST_SCANLINE: uint = 261;

const PALETTE: [u8 * 192] = [
    124,124,124,    0,0,252,        0,0,188,        68,40,188,
    148,0,132,      168,0,32,       168,16,0,       136,20,0,
    80,48,0,        0,120,0,        0,104,0,        0,88,0,
    0,64,88,        0,0,0,          0,0,0,          0,0,0,
    188,188,188,    0,120,248,      0,88,248,       104,68,252,
    216,0,204,      228,0,88,       248,56,0,       228,92,16,
    172,124,0,      0,184,0,        0,168,0,        0,168,68,
    0,136,136,      0,0,0,          0,0,0,          0,0,0,
    248,248,248,    60,188,252,     104,136,252,    152,120,248,
    248,120,248,    248,88,152,     248,120,88,     252,160,68,
    248,184,0,      184,248,24,     88,216,84,      88,248,152,
    0,232,216,      120,120,120,    0,0,0,          0,0,0,
    252,252,252,    164,228,252,    184,184,248,    216,184,248,
    248,184,248,    248,164,192,    240,208,176,    252,224,168,
    248,216,120,    216,248,120,    184,248,184,    184,248,216,
    0,252,252,      248,216,248,    0,0,0,          0,0,0
];

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
    fn vram_addr_increment(self) -> u16           { if (*self & 0x04) == 0 { 1 } else { 32 } }
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
    #[inline(always)]
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

    screen: ~([u8 * 184320]),  // 256 * 240 * 3
    scanline: u16,
    cy: u64
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
            4 => self.write_oamdata(val),
            5 => self.update_ppuscroll(val),
            6 => self.update_ppuaddr(val),
            7 => self.write_ppudata(val),
            _ => fail ~"can't happen"
        }
    }
}

#[deriving_eq]
pub struct StepResult {
    new_frame: bool,    // We wrapped around to the next scanline.
    vblank_nmi: bool,   // We entered VBLANK and must generate an NMI.
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
            oam: oam,

            screen: ~([ 0, ..184320 ]),
            scanline: 0,
            cy: 0
        }
    }

    //
    // Register manipulation
    //

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

    fn write_oamdata(&mut self, val: u8) {
        self.oam.storeb(self.regs.oam_addr as u16, val);
        self.regs.oam_addr += 1;
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

    //
    // Rendering
    //

    #[inline(always)]
    fn putpixel(&mut self, x: uint, y: uint, r: u8, g: u8, b: u8) {
        self.screen[(y * SCREEN_WIDTH + x) * 3 + 0] = r as u8;
        self.screen[(y * SCREEN_WIDTH + x) * 3 + 1] = g as u8;
        self.screen[(y * SCREEN_WIDTH + x) * 3 + 2] = b as u8;
    }

    fn render_scanline(&mut self) {
        // TODO: Scrolling, mirroring
        let mut nametable_offset = 0x2000 + 32 * (self.scanline / 8);
        for range(0, SCREEN_WIDTH) |x| {
            // TODO: Sprites, attributes

            // FIXME: For performance, we shouldn't be recomputing the tile for every pixel.
            let r, g, b;
            if self.regs.mask.show_background() {
                // Load the tile number from the nametable.
                let tile = self.vram.loadb(nametable_offset + (x as u16 / 8)) as u32;

                // Compute the pattern offset.
                let mut pattern_offset = (tile << 4) as u16 + (self.scanline as u16) % 8;
                pattern_offset += self.regs.ctrl.background_pattern_table_addr();

                // Determine the color of this pixel.
                let plane0 = self.vram.loadb(pattern_offset);
                let plane1 = self.vram.loadb(pattern_offset + 8);
                let bit0 = (plane0 >> (7 - ((x % 8) as u8))) & 1;
                let bit1 = (plane1 >> (7 - ((x % 8) as u8))) & 1;
                let color = (bit1 << 1) | bit0;

                // Fetch the palette from VRAM.
                // FIXME: Use the attribute to figure out which palette to use.
                let palette_index = self.vram.loadb(0x3f00 + (color as u16)) & 0x3f;

                r = PALETTE[palette_index * 3 + 0];
                g = PALETTE[palette_index * 3 + 1];
                b = PALETTE[palette_index * 3 + 2];
            } else {
                // FIXME: Use universal background color from palette.
                r = 0;
                g = 0;
                b = 0;
            }

            if self.regs.mask.show_sprites() {
                // TODO
            }

            self.putpixel(x, self.scanline as uint, r, g, b);
        }
    }

    fn start_vblank(&mut self, result: &mut StepResult) {
        self.regs.status.set_in_vblank(true);

        if self.regs.ctrl.vblank_nmi() {
            debug_print("VBLANK NMI!");
            result.vblank_nmi = true;
        }
    }

    fn step(&mut self, run_to_cycle: u64) -> StepResult {
        let mut result = StepResult { new_frame: false, vblank_nmi: false };
        loop {
            let next_scanline_cycle: u64 = self.cy + CYCLES_PER_SCANLINE;
            if next_scanline_cycle > run_to_cycle {
                break;
            }

            if self.scanline < (SCREEN_HEIGHT as u16) {
                self.render_scanline();
            }

            self.scanline += 1;
            if self.scanline == (VBLANK_SCANLINE as u16) {
                self.start_vblank(&mut result);
            } else if self.scanline == (LAST_SCANLINE as u16) { 
                result.new_frame = true;
                self.scanline = 0;
                self.regs.status.set_in_vblank(false);
            }

            self.cy += CYCLES_PER_SCANLINE;

            debug_assert(self.cy % CYCLES_PER_SCANLINE == 0, "at even scanline cycle");
        }

        return result;
    }
}

