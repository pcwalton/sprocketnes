//
// sprocketnes/mapper.rs
//
// Author: Patrick Walton
//

use rom::Rom;
use util;

use libc::{uint8_t, uint16_t};

#[deriving(PartialEq, Eq)]
pub enum MapperResult {
    Continue,
    Irq,
}

pub trait Mapper {
    fn prg_loadb(&mut self, addr: uint16_t) -> uint8_t;
    fn prg_storeb(&mut self, addr: uint16_t, val: uint8_t);
    fn chr_loadb(&mut self, addr: uint16_t) -> uint8_t;
    fn chr_storeb(&mut self, addr: uint16_t, val: uint8_t);
    fn next_scanline(&mut self) -> MapperResult;
}

pub fn create_mapper(rom: Box<Rom>) -> Box<Mapper+Send> {
    match rom.header.ines_mapper() {
        0 => {
            box Nrom {
                rom: rom,
            } as Box<Mapper+Send>
        },
        1 => box SxRom::new(rom) as Box<Mapper+Send>,
        4 => box TxRom::new(rom) as Box<Mapper+Send>,
        _ => panic!("unsupported mapper")
    }
}

//
// Mapper 0 (NROM)
//
// See http://wiki.nesdev.com/w/index.php/NROM
//

// TODO: RAM.
pub struct Nrom {
    pub rom: Box<Rom>,
}

impl Mapper for Nrom {
    fn prg_loadb(&mut self, addr: uint16_t) -> uint8_t {
        if addr < 0x8000 {
            0u8
        } else if self.rom.prg.len() > 16384 {
            self.rom.prg[addr as uint & 0x7fff]
        } else {
            self.rom.prg[addr as uint & 0x3fff]
        }
    }
    fn prg_storeb(&mut self, _: uint16_t, _: uint8_t) {}  // Can't store to PRG-ROM.
    fn chr_loadb(&mut self, addr: uint16_t) -> uint8_t { self.rom.chr[addr as uint] }
    fn chr_storeb(&mut self, _: uint16_t, _: uint8_t) {}  // Can't store to CHR-ROM.
    fn next_scanline(&mut self) -> MapperResult { MapperResult::Continue }
}

//
// Mapper 1 (SxROM/MMC1)
//
// See http://wiki.nesdev.com/w/index.php/Nintendo_MMC1
//

struct SxCtrl{ val: uint8_t }

impl Deref<uint8_t> for SxCtrl {
    fn deref(&self) -> &uint8_t {
        &self.val
    }
}

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

impl SxCtrl {
    fn prg_rom_mode(self) -> SxPrgBankMode {
        match (*self >> 2) & 3 {
            0 | 1 => SxPrgBankMode::Switch32K,
            2 => SxPrgBankMode::FixFirstBank,
            3 => SxPrgBankMode::FixLastBank,
            _ => panic!("can't happen")
        }
    }
}

struct SxRegs {
    ctrl: SxCtrl,   // $8000-$9FFF
    chr_bank_0: uint8_t, // $A000-$BFFF
    chr_bank_1: uint8_t, // $C000-$DFFF
    prg_bank: uint8_t,   // $E000-$FFFF
}

pub struct SxRom {
    rom: Box<Rom>,
    regs: SxRegs,
    // The internal accumulator.
    accum: uint8_t,
    // The write count. At the 5th write, we update the register.
    write_count: uint8_t,
    //prg_ram: Box<[uint8_t, ..8192]>,
    chr_ram: Box<[uint8_t, ..8192]>,
}

impl SxRom {
    fn new(rom: Box<Rom>) -> SxRom {
        SxRom {
            rom: rom,
            regs: SxRegs {
                ctrl: SxCtrl {
                    val: 3 << 2,
                },
                chr_bank_0: 0,
                chr_bank_1: 0,
                prg_bank: 0,
            },
            accum: 0,
            write_count: 0,
            //prg_ram: box() ([ 0, ..8192 ]),
            chr_ram: box() ([ 0, ..8192 ]),
        }
    }
}

impl Mapper for SxRom {
    fn prg_loadb(&mut self, addr: uint16_t) -> uint8_t {
        if addr < 0x8000 {
            0u8
        } else if addr < 0xc000 {
            let bank = match self.regs.ctrl.prg_rom_mode() {
                SxPrgBankMode::Switch32K => self.regs.prg_bank & 0xfe,
                SxPrgBankMode::FixFirstBank => 0,
                SxPrgBankMode::FixLastBank => self.regs.prg_bank,
            };
            self.rom.prg[(bank as uint * 16384) | ((addr & 0x3fff) as uint)]
        } else {
            let bank = match self.regs.ctrl.prg_rom_mode() {
                SxPrgBankMode::Switch32K => (self.regs.prg_bank & 0xfe) | 1,
                SxPrgBankMode::FixFirstBank => self.regs.prg_bank,
                SxPrgBankMode::FixLastBank => (*self.rom).header.prg_rom_size - 1,
            };
            self.rom.prg[(bank as uint * 16384) | ((addr & 0x3fff) as uint)]
        }
    }

    fn prg_storeb(&mut self, addr: uint16_t, val: uint8_t) {
        if addr < 0x8000 {
            return;
        }

        // Check the reset flag.
        if (val & 0x80) != 0 {
            self.write_count = 0;
            self.accum = 0;
            self.regs.ctrl = SxCtrl{val: *self.regs.ctrl | (3 << 2)};
            return;
        }

        // Write the lowest bit of the value into the right location of the accumulator.
        self.accum = self.accum | ((val & 1) << (self.write_count as uint));

        self.write_count += 1;
        if self.write_count == 5 {
            self.write_count = 0;

            // Write to the right internal register.
            if addr <= 0x9fff {
                self.regs.ctrl = SxCtrl{val: self.accum};
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
    fn chr_loadb(&mut self, addr: uint16_t) -> uint8_t     { self.chr_ram[addr as uint]       }
    fn chr_storeb(&mut self, addr: uint16_t, val: uint8_t) { self.chr_ram[addr as uint] = val }

    fn next_scanline(&mut self) -> MapperResult { MapperResult::Continue }
}

//
// Mapper 4 (TxROM/MMC3)
//
// See http://wiki.nesdev.com/w/index.php/MMC3
//

struct TxBankSelect{ val: uint8_t }

impl Deref<uint8_t> for TxBankSelect {
    fn deref(&self) -> &uint8_t {
        &self.val
    }
}

enum TxPrgBankMode {
    Swappable8000,
    SwappableC000,
}

impl TxBankSelect {
    fn bank_update_select(self) -> uint8_t { *self & 0x7 }
    fn prg_bank_mode(self) -> TxPrgBankMode {
        if (*self & 0x40) == 0 { TxPrgBankMode::Swappable8000 } else { TxPrgBankMode::SwappableC000 }
    }
    fn chr_a12_inversion(self) -> bool { (*self & 0x80) != 0 }
}

struct TxRegs {
    bank_select: TxBankSelect,  // Bank select (0x8000-0x9ffe even)
}

struct TxRom {
    rom: Box<Rom>,
    regs: TxRegs,
    prg_ram: Box<[uint8_t, ..8192]>,

    chr_banks_2k: [uint8_t, ..2],    // 2KB CHR-ROM banks
    chr_banks_1k: [uint8_t, ..4],    // 1KB CHR-ROM banks
    prg_banks:    [uint8_t, ..2],    // 8KB PRG-ROM banks

    scanline_counter: uint8_t,
    irq_reload: uint8_t,             // Copied into the scanline counter when it hits zero.
    irq_enabled: bool,
}

impl TxRom {
    fn new(rom: Box<Rom>) -> TxRom {
        TxRom {
            rom: rom,
            regs: TxRegs { bank_select: TxBankSelect{val: 0} },
            prg_ram: box() ([ 0, ..8192 ]),

            chr_banks_2k: [ 0, 0 ],
            chr_banks_1k: [ 0, 0, 0, 0 ],
            prg_banks: [ 0, 0 ],

            scanline_counter: 0,
            irq_reload: 0,
            irq_enabled: false,
        }
    }

    fn prg_bank_count(&self) -> uint8_t { self.rom.header.prg_rom_size * 2 }
}

impl Mapper for TxRom {
    fn prg_loadb(&mut self, addr: uint16_t) -> uint8_t {
        if addr < 0x6000 {
            0u8
        } else if addr < 0x8000 {
            self.prg_ram[addr as uint & 0x1fff]
        } else if addr < 0xa000 {
            // $8000-$9FFF might be switchable or fixed to the second to last bank.
            let bank = match self.regs.bank_select.prg_bank_mode() {
                TxPrgBankMode::Swappable8000 => self.prg_banks[0],
                TxPrgBankMode::SwappableC000 => self.prg_bank_count() - 2,
            };
            self.rom.prg[(bank as uint * 8192) | (addr as uint & 0x1fff)]
        } else if addr < 0xc000 {
            // $A000-$BFFF is switchable.
            self.rom.prg[(self.prg_banks[1] as uint * 8192) | (addr as uint & 0x1fff)]
        } else if addr < 0xe000 {
            // $C000-$DFFF might be switchable or fixed to the second to last bank.
            let bank = match self.regs.bank_select.prg_bank_mode() {
                TxPrgBankMode::Swappable8000 => self.prg_bank_count() - 2,
                TxPrgBankMode::SwappableC000 => self.prg_banks[0],
            };
            self.rom.prg[(bank as uint * 8192) | (addr as uint & 0x1fff)]
        } else {
            // $E000-$FFFF is fixed to the last bank.
            let bank = self.prg_bank_count() - 1;
            self.rom.prg[(bank as uint * 8192) | (addr as uint & 0x1fff)]
        }
    }

    fn prg_storeb(&mut self, addr: uint16_t, val: uint8_t) {
        if addr < 0x6000 {
            return;
        }

        if addr < 0x8000 {
            self.prg_ram[addr as uint & 0x1fff] = val;
        } else if addr < 0xa000 {
            if (addr & 1) == 0 {
                // Bank select.
                self.regs.bank_select = TxBankSelect{val: val};
            } else {
                // Bank data.
                let bank_update_select = self.regs.bank_select.bank_update_select() as uint;
                match bank_update_select {
                    0 ... 1 => self.chr_banks_2k[bank_update_select] = val,
                    2 ... 5 => self.chr_banks_1k[bank_update_select - 2] = val,
                    6 ... 7 => self.prg_banks[bank_update_select - 6] = val,
                    _ => panic!()
                }
            }
        } else if addr < 0xc000 {
            // TODO: Mirroring and PRG-RAM protect
        } else if addr < 0xe000 {
            if (addr & 1) == 0 {
                // IRQ latch.
                self.irq_reload = val;
            } else {
                // IRQ reload.
                self.scanline_counter = self.irq_reload;
            }
        } else {
            // IRQ enable.
            self.irq_enabled = (addr & 1) == 1;
        }
    }

    fn chr_loadb(&mut self, addr: uint16_t) -> uint8_t {
        let (bank, two_kb) = match (addr, self.regs.bank_select.chr_a12_inversion()) {
            (0x0000 ... 0x07ff, false) | (0x1000 ... 0x17ff, true) => (self.chr_banks_2k[0], true),
            (0x0800 ... 0x0fff, false) | (0x1800 ... 0x1fff, true) => (self.chr_banks_2k[1], true),
            (0x1000 ... 0x13ff, false) | (0x0000 ... 0x03ff, true) => (self.chr_banks_1k[0], false),
            (0x1400 ... 0x17ff, false) | (0x0400 ... 0x07ff, true) => (self.chr_banks_1k[1], false),
            (0x1800 ... 0x1bff, false) | (0x0800 ... 0x0bff, true) => (self.chr_banks_1k[2], false),
            (0x1c00 ... 0x1fff, false) | (0x0c00 ... 0x0fff, true) => (self.chr_banks_1k[3], false),
            _ => return 0,
        };
        if two_kb {
            self.rom.chr[(bank as uint * 1024) + (addr as uint & 0x7ff)]
        } else {
            self.rom.chr[(bank as uint * 1024) | (addr as uint & 0x3ff)]
        }
    }

    fn chr_storeb(&mut self, _: uint16_t, _: uint8_t) {
        // TODO: CHR-RAM
    }

    fn next_scanline(&mut self) -> MapperResult {
        if self.scanline_counter != 0 {
            self.scanline_counter -= 1;
            if self.scanline_counter == 0 {
                self.scanline_counter = self.irq_reload;

                if self.irq_enabled {
                    util::debug_print("*** Generated IRQ! ***");
                    return MapperResult::Irq;
                }
            }
        }
        MapperResult::Continue
    }
}
