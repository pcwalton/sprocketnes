//
// Author: Patrick Walton
//

use mapper::{Mapper, MapperResult};
use mem::Mem;
use util::Save;

use std::cell::RefCell;
use std::fs::File;
use std::rc::Rc;
use std::ops::{Deref, DerefMut};


pub const SCREEN_WIDTH: usize = 256;
pub const SCREEN_HEIGHT: usize = 240;
pub const CYCLES_PER_SCANLINE: u64 = 114;   // 29781 cycles per frame / 261 scanlines
pub const VBLANK_SCANLINE: usize = 241;
pub const LAST_SCANLINE: usize = 261;

static PALETTE: [u8; 192] = [
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

#[derive(Copy, Clone)]
struct Regs {
    ctrl: PpuCtrl,      // PPUCTRL: 0x2000
    mask: PpuMask,      // PPUMASK: 0x2001
    status: PpuStatus,  // PPUSTATUS: 0x2002
    oam_addr: u8,       // OAMADDR: 0x2003
    scroll: PpuScroll,  // PPUSCROLL: 0x2005
    addr: PpuAddr,      // PPUADDR: 0x2006
}

save_struct!(Regs { ctrl, mask, status, oam_addr, scroll, addr });

//
// PPUCTRL: 0x2000
//

#[derive(Copy, Clone)]
struct PpuCtrl{ val: u8 }

enum SpriteSize {
    SpriteSize8x8,
    SpriteSize8x16
}

impl Deref for PpuCtrl {
    type Target = u8;

    fn deref(&self) -> &u8 {
        &self.val
    }
}

impl DerefMut for PpuCtrl {
    fn deref_mut(&mut self) -> &mut u8 {
        &mut self.val
    }
}

impl PpuCtrl {
    fn x_scroll_offset(self) -> u16               { if (*self & 0x01) == 0 { 0 } else { 256 } }
    fn y_scroll_offset(self) -> u16               { if (*self & 0x02) == 0 { 0 } else { 240 } }
    fn vram_addr_increment(self) -> u16           { if (*self & 0x04) == 0 { 1 } else { 32 } }
    fn sprite_pattern_table_addr(self) -> u16     { if (*self & 0x08) == 0 { 0 } else { 0x1000 } }
    fn background_pattern_table_addr(self) -> u16 { if (*self & 0x10) == 0 { 0 } else { 0x1000 } }
    fn sprite_size(self) -> SpriteSize {
        if (*self & 0x20) == 0 { SpriteSize::SpriteSize8x8 } else { SpriteSize::SpriteSize8x16 }
    }
    fn vblank_nmi(self) -> bool                   { (*self & 0x80) != 0 }
}

//
// PPUMASK: 0x2001
//

#[derive(Copy, Clone)]
struct PpuMask {val: u8 }

impl Deref for PpuMask {
    type Target = u8;

    fn deref(&self) -> &u8 {
        &self.val
    }
}

impl DerefMut for PpuMask {
    fn deref_mut(&mut self) -> &mut u8 {
        &mut self.val
    }
}

impl PpuMask {
    // 0x01: grayscale
    // 0x02: show background on left
    // 0x04: show sprites on left
    fn show_background(self) -> bool         { (*self & 0x08) != 0 }
    fn show_sprites(self) -> bool            { (*self & 0x10) != 0 }
    // 0x20: intensify reds
    // 0x40: intensify greens
    // 0x80: intensify blues
}

//
// PPUSTATUS: 0x2002
//

#[derive(Copy, Clone)]
struct PpuStatus { val: u8 }

impl Deref for PpuStatus {
    type Target = u8;

    fn deref(&self) -> &u8 {
        &self.val
    }
}

impl DerefMut for PpuStatus {
    fn deref_mut(&mut self) -> &mut u8 {
        &mut self.val
    }
}

impl PpuStatus {
    // TODO: open bus junk in bits [0,5)
    fn set_sprite_overflow(&mut self, val: bool) {
        *self = if val { PpuStatus{ val: **self | 0x20 } }
        else { PpuStatus{ val: **self & !0x20} }
    }
    fn set_sprite_zero_hit(&mut self, val: bool) {
        *self = if val { PpuStatus{ val: **self | 0x40 } }
        else { PpuStatus{ val: **self & !0x40} }
    }
    fn set_in_vblank(&mut self, val: bool) {
        *self = if val { PpuStatus{ val: **self | 0x80 } }
        else { PpuStatus{ val: **self & !0x80} }
    }
}

//
// PPUSCROLL: 0x2005
//

#[derive(Copy, Clone)]
struct PpuScroll {
    x: u8,
    y: u8,
    next: PpuScrollDir
}

save_struct!(PpuScroll { x, y, next });

#[derive(Copy, Clone)]
enum PpuScrollDir {
    XDir,
    YDir,
}

save_enum!(PpuScrollDir { XDir, YDir });

//
// PPUADDR: 0x2006
//

#[derive(Copy, Clone)]
struct PpuAddr {
    val: u16,
    next: PpuAddrByte
}

save_struct!(PpuAddr { val, next });

#[derive(Copy, Clone)]
enum PpuAddrByte {
    Hi,
    Lo,
}

save_enum!(PpuAddrByte { Hi, Lo });

// PPU VRAM. This implements the same Mem trait that the CPU memory does.

pub struct Vram {
    pub mapper: Rc<RefCell<Box<Mapper+Send>>>,
    pub nametables: [u8; 0x800],  // 2 nametables, 0x400 each. FIXME: Not correct for all mappers.
    pub palette: [u8; 0x20],
}

impl Vram {
    pub fn new(mapper: Rc<RefCell<Box<Mapper+Send>>>) -> Vram {
        Vram {
            mapper: mapper,
            nametables: [ 0; 0x800 ],
            palette: [ 0; 0x20 ]
        }
    }
}

impl Mem for Vram {
    #[inline(always)]
    fn loadb(&mut self, addr: u16) -> u8 {
        if addr < 0x2000 {          // Tilesets 0 or 1
            let mut mapper = self.mapper.borrow_mut();
            mapper.chr_loadb(addr)
        } else if addr < 0x3f00 {   // Name table area
            self.nametables[addr as usize & 0x07ff]
        } else if addr < 0x4000 {   // Palette area
            self.palette[addr as usize & 0x1f]
        } else {
            panic!("invalid VRAM read")
        }
    }
    fn storeb(&mut self, addr: u16, val: u8) {
        if addr < 0x2000 {
            let mut mapper = self.mapper.borrow_mut();
            mapper.chr_storeb(addr, val)
        } else if addr < 0x3f00 {           // Name table area
            let addr = addr & 0x07ff;
            self.nametables[addr as usize] = val;
        } else if addr < 0x4000 {   // Palette area
            let mut addr = addr & 0x1f;
            if addr == 0x10 {
                addr = 0x00;    // Mirror sprite background color into universal background color.
            }
            self.palette[addr as usize] = val;
        }
    }
}

impl Save for Vram {
    fn save(&mut self, fd: &mut File) {
        let mut nametables: &mut [u8] = &mut self.nametables;
        nametables.save(fd);
        let mut palette: &mut [u8] = &mut self.palette;
        palette.save(fd);
    }
    fn load(&mut self, fd: &mut File) {
        let mut nametables: &mut [u8] = &mut self.nametables;
        nametables.load(fd);
        let mut palette: &mut [u8] = &mut self.palette;
        palette.load(fd);
    }
}

//
// Object Attribute Memory (OAM)
//

pub struct Oam {
    pub oam: [u8; 0x100]
}

impl Oam {
    pub fn new() -> Oam {
        Oam { oam: [ 0; 0x100 ] }
    }
}

impl Mem for Oam {
    fn loadb(&mut self, addr: u16) -> u8     { self.oam[addr as usize] }
    fn storeb(&mut self, addr: u16, val: u8) { self.oam[addr as usize] = val }
}

impl Save for Oam {
    fn save(&mut self, fd: &mut File) {
        let mut oam: &mut [u8] = &mut self.oam;
        oam.save(fd);
    }
    fn load(&mut self, fd: &mut File) {
        let mut oam: &mut [u8] = &mut self.oam;
        oam.load(fd);
    }
}

struct SpriteStruct {
    x: u8,
    y: u8,
    tile_index_byte: u8,
    attribute_byte: u8,
}

// Specifies the indices of the tiles that make up this sprite.
enum SpriteTiles {
    SpriteTiles8x8(u16),
    SpriteTiles8x16(u16, u16)
}

use self::SpriteTiles::*;

impl SpriteStruct {
    fn tiles(&self, ppu: &Ppu) -> SpriteTiles {
        let base = ppu.regs.ctrl.sprite_pattern_table_addr();
        match ppu.regs.ctrl.sprite_size() {
            SpriteSize::SpriteSize8x8 => SpriteTiles8x8(self.tile_index_byte as u16 | base),
            SpriteSize::SpriteSize8x16 => {
                // We ignore the base set in PPUCTRL here.
                let mut first = (self.tile_index_byte & !1) as u16;
                if (self.tile_index_byte & 1) != 0 {
                    first += 0x1000;
                }
                SpriteTiles8x16(first, first + 1)
            }
        }
    }

    fn palette(&self) -> u8                 { (self.attribute_byte & 3) + 4 }
    fn flip_horizontal(&self) -> bool       { (self.attribute_byte & 0x40) != 0 }
    fn flip_vertical(&self) -> bool         { (self.attribute_byte & 0x80) != 0 }

    fn priority(&self) -> SpritePriority {
        if (self.attribute_byte & 0x20) == 0 { AboveBg } else { BelowBg }
    }

    // Quick test to see whether this sprite is on the given scanline.
    fn on_scanline(&self, ppu: &Ppu, y: u8) -> bool {
        if y < self.y { return false; }
        match ppu.regs.ctrl.sprite_size() {
            SpriteSize::SpriteSize8x8 => y < self.y + 8,
            SpriteSize::SpriteSize8x16 => y < self.y + 16
        }
    }

    // Quick test to see whether the given point is in the bounding box of this sprite.
    fn in_bounding_box(&self, ppu: &Ppu, x: u8, y: u8) -> bool {
        x >= self.x && x < self.x + 8 && self.on_scanline(ppu, y)
    }
}

// The main PPU structure. This structure is separate from the PPU memory just as the CPU is.

pub struct Ppu {
    regs: Regs,
    vram: Vram,
    oam: Oam,

    pub screen: Box<[u8; 184320]>,  // 256 * 240 * 3
    scanline: u16,
    ppudata_buffer: u8,

    // NB: These two cannot always be computed from PPUCTRL and PPUSCROLL, because PPUADDR *also*
    // updates the scroll position. This is important to emulate.
    scroll_x: u16,
    scroll_y: u16,

    cy: u64
}

impl Mem for Ppu {
    // Performs a load of the PPU register at the given CPU address.
    fn loadb(&mut self, addr: u16) -> u8 {
        debug_assert!(addr >= 0x2000 && addr < 0x4000, "invalid PPU register");
        match addr & 7 {
            0 => *self.regs.ctrl,
            1 => *self.regs.mask,
            2 => self.read_ppustatus(),
            3 => 0, // OAMADDR is read-only
            4 => panic!("OAM read unimplemented"),
            5 => 0, // PPUSCROLL is read-only
            6 => 0, // PPUADDR is read-only
            7 => self.read_ppudata(),
            _ => panic!("can't happen")
        }
    }

    // Performs a store to the PPU register at the given CPU address.
    fn storeb(&mut self, addr: u16, val: u8) {
        debug_assert!(addr >= 0x2000 && addr < 0x4000, "invalid PPU register");
        match addr & 7 {
            0 => self.update_ppuctrl(val),
            1 => self.regs.mask = PpuMask{val: val},
            2 => (),    // PPUSTATUS is read-only
            3 => self.regs.oam_addr = val,
            4 => self.write_oamdata(val),
            5 => self.update_ppuscroll(val),
            6 => self.update_ppuaddr(val),
            7 => self.write_ppudata(val),
            _ => panic!("can't happen")
        }
    }
}

#[derive(PartialEq, Eq)]
pub struct StepResult {
    pub new_frame: bool,    // We wrapped around to the next scanline.
    pub vblank_nmi: bool,   // We entered VBLANK and must generate an NMI.
    pub scanline_irq: bool, // The mapper wants to execute a scanline IRQ.
}

#[derive(Copy, Clone)]
struct Rgb {
    r: u8,
    g: u8,
    b: u8,
}

enum PatternPixelKind {
    Background,
    Sprite,
}

struct NametableAddr {
    base: u16,
    x_index: u8,
    y_index: u8,
}

struct SpriteColor {
    priority: SpritePriority,
    color: Rgb,
}

enum SpritePriority {
    AboveBg,
    BelowBg,
}

use self::SpritePriority::*;

impl Save for Ppu {
    fn save(&mut self, fd: &mut File) {
        self.regs.save(fd);
        self.vram.save(fd);
        self.oam.save(fd);
        self.scanline.save(fd);
        self.ppudata_buffer.save(fd);
        self.scroll_x.save(fd);
        self.scroll_y.save(fd);
        self.cy.save(fd);
    }
    fn load(&mut self, fd: &mut File) {
        self.regs.load(fd);
        self.vram.load(fd);
        self.oam.load(fd);
        self.scanline.load(fd);
        self.ppudata_buffer.load(fd);
        self.scroll_x.load(fd);
        self.scroll_y.load(fd);
        self.cy.load(fd);
    }
}

impl Ppu {
    pub fn new(vram: Vram, oam: Oam) -> Ppu {
        Ppu {
            regs: Regs {
                ctrl: PpuCtrl{val: 0},
                mask: PpuMask{val: 0},
                status: PpuStatus{val:0},
                oam_addr: 0,
                scroll: PpuScroll { x: 0, y: 0, next: PpuScrollDir::XDir },
                addr: PpuAddr { val: 0, next: PpuAddrByte::Hi },
            },
            vram: vram,
            oam: oam,

            screen: Box::new([ 0; 184320 ]),
            scanline: 0,
            ppudata_buffer: 0,

            scroll_x: 0,
            scroll_y: 0,

            cy: 0
        }
    }

    //
    // Color utilities
    //

    #[inline(always)]
    fn get_color(&self, palette_index: u8) -> Rgb {
        Rgb {
            r: PALETTE[palette_index as usize * 3 + 2],
            g: PALETTE[palette_index as usize * 3 + 1],
            b: PALETTE[palette_index as usize * 3 + 0],
        }
    }

    //
    // Register manipulation
    //

    fn update_ppuctrl(&mut self, val: u8) {
        self.regs.ctrl = PpuCtrl{val:val};

        self.scroll_x = (self.scroll_x & 0xff) | self.regs.ctrl.x_scroll_offset();
        self.scroll_y = (self.scroll_y & 0xff) | self.regs.ctrl.y_scroll_offset();
    }

    fn update_ppuscroll(&mut self, val: u8) {
        match self.regs.scroll.next {
            PpuScrollDir::XDir => {
                self.scroll_x = (self.scroll_x & 0xff00) | (val as u16);

                self.regs.scroll.x = val;
                self.regs.scroll.next = PpuScrollDir::YDir;
            }
            PpuScrollDir::YDir => {
                self.scroll_y = (self.scroll_y & 0xff00) | (val as u16);

                self.regs.scroll.y = val;
                self.regs.scroll.next = PpuScrollDir::XDir;
            }
        }
    }

    fn write_oamdata(&mut self, val: u8) {
        self.oam.storeb(self.regs.oam_addr as u16, val);
        self.regs.oam_addr += 1;
    }

    fn update_ppuaddr(&mut self, val: u8) {
        match self.regs.addr.next {
            PpuAddrByte::Hi => {
                self.regs.addr.val = (self.regs.addr.val & 0x00ff) | ((val as u16) << 8);
                self.regs.addr.next = PpuAddrByte::Lo;
            }
            PpuAddrByte::Lo => {
                self.regs.addr.val = (self.regs.addr.val & 0xff00) | (val as u16);
                self.regs.addr.next = PpuAddrByte::Hi;

                // Adjust the scroll registers.
                // TODO: This is pretty much a hack. The right way is to precisely emulate the PPU
                // internal registers.
                // TODO: Y scrolling.
                let addr = self.regs.addr.val & 0x07ff;
                let xscroll_base = if addr < 0x400 { 0 } else { 256 };
                self.scroll_x = (self.scroll_x & 0xff) | xscroll_base;
            }
        }
    }

    fn read_ppustatus(&mut self) -> u8 {
        // Reset latch.
        self.regs.scroll.next = PpuScrollDir::XDir;
        self.regs.addr.next = PpuAddrByte::Hi;

        *self.regs.status
    }

    fn write_ppudata(&mut self, val: u8) {
        self.vram.storeb(self.regs.addr.val, val);
        self.regs.addr.val += self.regs.ctrl.vram_addr_increment();
    }

    fn read_ppudata(&mut self) -> u8 {
        let addr = self.regs.addr.val;
        let val = self.vram.loadb(addr);
        self.regs.addr.val += self.regs.ctrl.vram_addr_increment();

        // Emulate the PPU buffering quirk.
        if addr < 0x3f00 {
            let buffered_val = self.ppudata_buffer;
            self.ppudata_buffer = val;
            buffered_val
        } else {
            val
        }
    }

    //
    // Background rendering helpers
    //

    fn nametable_addr(&mut self, mut x_index: u16, mut y_index: u16) -> NametableAddr {
        x_index %= 64;
        y_index %= 60;

        let nametable_base = match (x_index >= 32, y_index >= 30) {
            (false, false)  => 0x2000,
            (true, false)   => 0x2400,
            (false, true)   => 0x2800,
            (true, true)    => 0x2c00,
        };

        NametableAddr {
            base: nametable_base,
            x_index: (x_index % 32) as u8,
            y_index: (y_index % 30) as u8
        }
    }

    #[inline(always)]
    fn make_sprite_info(&mut self, index: u16) -> SpriteStruct {
        SpriteStruct {
            y: self.oam.loadb(index * 4 + 0) + 1,
            tile_index_byte: self.oam.loadb(index * 4 + 1),
            attribute_byte: self.oam.loadb(index * 4 + 2),
            x: self.oam.loadb(index * 4 + 3),
        }
    }

    #[inline(always)]
    fn each_sprite<F>(&mut self, mut f: F)
        where F: FnMut(&mut Ppu, &SpriteStruct, u8) -> bool{
        for i in 0..64 {
            let sprite = self.make_sprite_info(i as u16);
            if !f(self, &sprite, i as u8) {
                return
            }
        }
    }

    //
    // Rendering
    //

    #[inline(always)]
    fn putpixel(&mut self, x: usize, y: usize, color: Rgb) {
        self.screen[(y * SCREEN_WIDTH + x) * 3 + 0] = color.r;
        self.screen[(y * SCREEN_WIDTH + x) * 3 + 1] = color.g;
        self.screen[(y * SCREEN_WIDTH + x) * 3 + 2] = color.b;
    }

    // Returns the color (pre-palette lookup) of pixel (x,y) within the given tile.
    #[inline(always)]
    fn get_pattern_pixel(&mut self, kind: PatternPixelKind, tile: u16, x: u8, y: u8) -> u8 {
        // Compute the pattern offset.
        let mut pattern_offset = (tile << 4) + (y as u16);
        match kind {
            PatternPixelKind::Background => pattern_offset += self.regs.ctrl.background_pattern_table_addr(),
            PatternPixelKind::Sprite     => pattern_offset += self.regs.ctrl.sprite_pattern_table_addr(),
        }

        // Determine the color of this pixel.
        let plane0 = self.vram.loadb(pattern_offset);
        let plane1 = self.vram.loadb(pattern_offset + 8);
        let bit0 = (plane0 >> ((7 - ((x % 8) as u8)) as usize)) & 1;
        let bit1 = (plane1 >> ((7 - ((x % 8) as u8)) as usize)) & 1;
        (bit1 << 1) | bit0
    }

    // Returns true if the background was opaque here, false otherwise.
    #[inline(always)]
    fn get_background_pixel(&mut self, x: u8) -> Option<Rgb> {
        // Adjust X and Y to account for scrolling.
        let x = x as u16 + self.scroll_x;
        let y = self.scanline as u16 + self.scroll_y;

        // Compute the nametable address, tile index, and pixel offset within that tile.
        let NametableAddr { base, x_index, y_index } = self.nametable_addr(x / 8, y / 8);
        let (xsub, ysub) = ((x % 8) as u8, (y % 8) as u8);

        // Compute the nametable address and load the tile number from the nametable.
        let tile = self.vram.loadb(base + 32 * (y_index as u16) + (x_index as u16));

        // Fetch the pattern color.
        let pattern_color = self.get_pattern_pixel(PatternPixelKind::Background, tile as u16, xsub, ysub);
        if pattern_color == 0 {
            return None;    // Transparent.
        }

        // Now load the attribute bits from the attribute table.
        let group = y_index / 4 * 8 + x_index / 4;
        let attr_byte = self.vram.loadb(base + 0x3c0 + (group as u16));
        let (left, top) = (x_index % 4 < 2, y_index % 4 < 2);
        let attr_table_color = match (left, top) {
            (true, true) => attr_byte & 0x3,
            (false, true) => (attr_byte >> 2) & 0x3,
            (true, false) => (attr_byte >> 4) & 0x3,
            (false, false) => (attr_byte >> 6) & 0x3
        };

        // Determine the final color and fetch the palette from VRAM.
        let tile_color = (attr_table_color << 2) | pattern_color;
        let palette_index = self.vram.loadb(0x3f00 + (tile_color as u16)) & 0x3f;
        return Some(self.get_color(palette_index));
    }

    fn get_sprite_pixel(&mut self,
                        visible_sprites: &[Option<u8>; 8],
                        x: u8,
                        background_opaque: bool)
                     -> Option<SpriteColor> {
        for &visible_sprite_opt in visible_sprites.iter() {
            match visible_sprite_opt {
                None => return None,
                Some(index) => {
                    let sprite = self.make_sprite_info(index as u16);

                    // Don't need to consider this sprite if we aren't in its bounding box.
                    if !sprite.in_bounding_box(self, x as u8, self.scanline as u8) {
                        continue
                    }

                    let pattern_color;
                    match sprite.tiles(self) {
                        // TODO: 8x16 rendering
                        SpriteTiles8x8(tile) | SpriteTiles8x16(tile, _) => {
                            let mut x = x - sprite.x;
                            if sprite.flip_horizontal() { x = 7 - x; }

                            let mut y = self.scanline as u8 - sprite.y;
                            if sprite.flip_vertical() { y = 7 - y; }

                            debug_assert!(x < 8, "sprite X miscalculation");
                            debug_assert!(y < 8, "sprite Y miscalculation");

                            pattern_color = self.get_pattern_pixel(PatternPixelKind::Sprite, tile, x, y);
                        }
                    }

                    // If the pattern color was zero, this part of the sprite is transparent.
                    if pattern_color == 0 {
                        continue
                    }

                    // OK, so we know this pixel is opaque. Now if this is the first sprite and the
                    // background was not transparent, set sprite 0 hit.
                    if index == 0 && background_opaque {
                        self.regs.status.set_sprite_zero_hit(true);
                    }

                    // Determine final tile color and do the palette lookup.
                    let tile_color = (sprite.palette() << 2) | pattern_color;
                    let palette_index = self.vram.loadb(0x3f00 + (tile_color as u16)) & 0x3f;
                    let final_color = self.get_color(palette_index);

                    return Some(SpriteColor { priority: sprite.priority(), color: final_color });
                }
            }
        }
        return None;
    }

    fn compute_visible_sprites(&mut self) -> [Option<u8>; 8] {
        let mut count = 0;
        let mut result = [None; 8];
        self.each_sprite(|this, sprite, index| {
            if sprite.on_scanline(this, this.scanline as u8) {
                if count < 8 {
                    result[count] = Some(index);
                    count += 1;
                    true
                } else {
                    this.regs.status.set_sprite_overflow(true);
                    false
                }
            } else {
                true
            }
        });
        result
    }

    fn render_scanline(&mut self) {
        // TODO: Scrolling, mirroring
        let visible_sprites = self.compute_visible_sprites();

        let backdrop_color_index = self.vram.loadb(0x3f00) & 0x3f;
        let backdrop_color = self.get_color(backdrop_color_index);

        for x in 0..SCREEN_WIDTH {
            // FIXME: For performance, we shouldn't be recomputing the tile for every pixel.
            let mut background_color = None;
            if self.regs.mask.show_background() {
                background_color = self.get_background_pixel(x as u8);
            }

            let mut sprite_color = None;
            if self.regs.mask.show_sprites() {
                sprite_color = self.get_sprite_pixel(&visible_sprites,
                                                     x as u8,
                                                     background_color.is_some());
            }

            // Combine colors using priority.
            let color = match (background_color, sprite_color) {
                (None, None) => backdrop_color,
                (Some(color), None) => color,
                (Some(color), Some(SpriteColor { priority: BelowBg, .. })) => color,
                (None, Some(SpriteColor { priority: BelowBg, color })) => color,
                (_, Some(SpriteColor { priority: AboveBg, color })) => color,
            };

            let scanline = self.scanline;
            self.putpixel(x, scanline as usize, color);
        }
    }

    fn start_vblank(&mut self, result: &mut StepResult) {
        self.regs.status.set_in_vblank(true);

        // FIXME: Is this correct? Or does it happen on the *next* frame?
        self.regs.status.set_sprite_zero_hit(false);

        if self.regs.ctrl.vblank_nmi() {
            result.vblank_nmi = true;
        }
    }

    #[inline(never)]
    pub fn step(&mut self, run_to_cycle: u64) -> StepResult {
        let mut result = StepResult { new_frame: false, vblank_nmi: false, scanline_irq: false };
        loop {
            let next_scanline_cycle: u64 = self.cy + CYCLES_PER_SCANLINE;
            if next_scanline_cycle > run_to_cycle {
                break;
            }

            if self.scanline < (SCREEN_HEIGHT as u16) {
                self.render_scanline();
            }

            self.scanline += 1;

            {
                let mut mapper = self.vram.mapper.borrow_mut();
                if mapper.next_scanline() == MapperResult::Irq {
                    result.scanline_irq = true
                }
            }

            if self.scanline == (VBLANK_SCANLINE as u16) {
                self.start_vblank(&mut result);
            } else if self.scanline == (LAST_SCANLINE as u16) {
                result.new_frame = true;
                self.scanline = 0;
                self.regs.status.set_in_vblank(false);
            }

            self.cy += CYCLES_PER_SCANLINE;

            debug_assert!(self.cy % CYCLES_PER_SCANLINE == 0, "at even scanline cycle");
        }

        return result;
    }
}
