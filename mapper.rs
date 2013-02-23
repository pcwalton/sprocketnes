//
// sprocketnes/mapper.rs
//
// Author: Patrick Walton
//

use mem::Mem;
use rom::Rom;

use core::cast::transmute;

pub trait Mapper {
    fn prg_loadb(&mut self, addr: u16) -> u8;
    fn prg_storeb(&mut self, addr: u16, val: u8);
    fn chr_loadb(&mut self, addr: u16) -> u8;
    fn chr_storeb(&mut self, addr: u16, val: u8);
}

impl Mapper {
    static fn with_mapper<R>(rom: ~Rom, f: &fn(&Mapper) -> R) -> R {
        match rom.header.mapper() {
            0 => {
                unsafe {
                    let mut nrom = Nrom { rom: rom };
                    let mut nrom_ptr: &static/Nrom = transmute(&mut nrom);  // FIXME: Wat?
                    f(nrom_ptr as &Mapper)
                }
            },
            1 => {
                unsafe {
                    let mut sxrom = SxRom::new(rom);
                    let sxrom_ptr: &static/SxRom = transmute(&mut sxrom);   // FIXME: Wat?
                    f(sxrom_ptr as &Mapper)
                }
            }
            _ => fail!(~"unsupported mapper")
        }
    }
}

//
// Mapper 0 (NROM)
//
// See http://wiki.nesdev.com/w/index.php/NROM
//

// TODO: RAM.
pub struct Nrom {
    rom: ~Rom,
}

impl Mapper for Nrom {
    fn prg_loadb(&mut self, addr: u16) -> u8 {
        if addr < 0x8000 {
            0
        } else if self.rom.prg.len() > 16384 {
            self.rom.prg[addr & 0x7fff]
        } else {
            self.rom.prg[addr & 0x3fff]
        }
    }
    fn prg_storeb(&mut self, _: u16, _: u8) {}  // Can't store to PRG-ROM.
    fn chr_loadb(&mut self, addr: u16) -> u8 { self.rom.chr[addr] }
    fn chr_storeb(&mut self, _: u16, _: u8) {}  // Can't store to CHR-ROM.
}

//
// Mapper 1 (SxROM/MMC1)
//
// See http://wiki.nesdev.com/w/index.php/Nintendo_MMC1
//

struct SxCtrl(u8);

pub enum Mirroring {
    OneScreenLower,
    OneScreenUpper,
    Vertical,
    Horizontal,
}

enum SxPrgBankMode {
    Switch32K,      // Switch 32K at $8000, ignore low bit
    FixFirstBank,   // Fix first bank at $8000, switch 16K bank at $C000
    FixLastBank,    // Fix last bank at $C000, switch 16K bank at $8000
}

enum SxChrBankMode {
    Switch8K,       // Switch 8K at a time
    SwitchTwo4K,    // Switch two separate 4K banks
}

impl SxCtrl {
    fn mirroring(self) -> Mirroring {
        match *self & 3 {
            0 => OneScreenLower,
            1 => OneScreenUpper,
            2 => Vertical,
            3 => Horizontal,
            _ => fail!(~"can't happen")
        }
    }
    fn prg_rom_mode(self) -> SxPrgBankMode {
        match (*self >> 2) & 3 {
            0 | 1 => Switch32K,
            2 => FixFirstBank,
            3 => FixLastBank,
            _ => fail!(~"can't happen")
        }
    }
    fn chr_rom_mode(self) -> SxChrBankMode {
        if ((*self >> 4) & 1) == 0 { Switch8K } else { SwitchTwo4K }
    }
}

pub struct SxRegs {
    ctrl: SxCtrl,   // $8000-$9FFF
    chr_bank_0: u8, // $A000-$BFFF
    chr_bank_1: u8, // $C000-$DFFF
    prg_bank: u8,   // $E000-$FFFF
}

pub struct SxRom {
    rom: ~Rom,
    regs: SxRegs,
    // The internal accumulator.
    accum: u8,
    // The write count. At the 5th write, we update the register.
    write_count: u8,
    prg_ram: ~([u8 * 8192]),
    chr_ram: ~([u8 * 8192]),
}

impl SxRom {
    static fn new(rom: ~Rom) -> SxRom {
        SxRom {
            rom: rom,
            regs: SxRegs {
                ctrl: SxCtrl(3 << 2),
                chr_bank_0: 0,
                chr_bank_1: 0,
                prg_bank: 0,
            },
            accum: 0,
            write_count: 0,
            prg_ram: ~([ 0, ..8192 ]),
            chr_ram: ~([ 0, ..8192 ]),
        }
    }
}

impl Mapper for SxRom {
    fn prg_loadb(&mut self, addr: u16) -> u8 {
        unsafe {
            if addr < 0x8000 {
                0
            } else if addr < 0xc000 {
                let bank = match self.regs.ctrl.prg_rom_mode() {
                    Switch32K => self.regs.prg_bank & 0xfe,
                    FixFirstBank => 0,
                    FixLastBank => self.regs.prg_bank,
                };
                self.rom.prg[(bank as uint * 16384) | ((addr & 0x3fff) as uint)]
            } else {
                let bank = match self.regs.ctrl.prg_rom_mode() {
                    Switch32K => (self.regs.prg_bank & 0xfe) | 1,
                    FixFirstBank => self.regs.prg_bank,
                    FixLastBank => (*self.rom).header.prg_rom_size - 1,
                };
                self.rom.prg[(bank as uint * 16384) | ((addr & 0x3fff) as uint)]
            }
        }
    }

    fn prg_storeb(&mut self, addr: u16, val: u8) {
        if addr < 0x8000 {
            return;
        }

        // Check the reset flag.
        if (val & 0x80) != 0 {
            self.write_count = 0;
            self.accum = 0;
            self.regs.ctrl = SxCtrl(*self.regs.ctrl | (3 << 2));
            return;
        }

        // Write the lowest bit of the value into the right location of the accumulator.
        self.accum = self.accum | ((val & 1) << self.write_count);

        self.write_count += 1;
        if self.write_count == 5 {
            self.write_count = 0;

            // Write to the right internal register.
            if addr <= 0x9fff {
                self.regs.ctrl = SxCtrl(self.accum);
            } else if addr <= 0xbfff {
                self.regs.chr_bank_0 = self.accum;
            } else if addr <= 0xdfff {
                self.regs.chr_bank_1 = self.accum;
            } else {
                self.regs.prg_bank = self.accum;
            }

            self.accum = 0;
        }
    }

    // FIXME: Apparently this mapper can have CHR-ROM as well. Handle this case.
    fn chr_loadb(&mut self, addr: u16) -> u8     { self.chr_ram[addr]       }
    fn chr_storeb(&mut self, addr: u16, val: u8) { self.chr_ram[addr] = val }
}

