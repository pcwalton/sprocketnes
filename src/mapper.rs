//! Defines the `Mapper` trait and mapper implementations that are used to translate CPU addresses
//! to addresses on the cartridge memory.

//
// Author: Patrick Walton
//

use rom::Rom;

use std::ops::Deref;

#[derive(PartialEq, Eq)]
pub enum MapperResult {
    Continue,
    Irq,
}

pub trait Mapper {
    fn prg_loadb(&mut self, addr: u16) -> u8;
    fn prg_storeb(&mut self, addr: u16, val: u8);
    fn chr_loadb(&mut self, addr: u16) -> u8;
    fn chr_storeb(&mut self, addr: u16, val: u8);
    fn next_scanline(&mut self) -> MapperResult;
}

pub fn create_mapper(rom: Box<Rom>) -> Box<Mapper+Send> {
    match rom.header.ines_mapper() {
        0 => {
            Box::new(Nrom {
                rom: rom,
            }) as Box<Mapper+Send>
        },
        1 => Box::new(SxRom::new(rom)) as Box<Mapper+Send>,
        4 => Box::new(TxRom::new(rom)) as Box<Mapper+Send>,
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
    fn prg_loadb(&mut self, addr: u16) -> u8 {
        if addr < 0x8000 {
            0u8
        } else if self.rom.prg.len() > 16384 {
            self.rom.prg[addr as usize & 0x7fff]
        } else {
            self.rom.prg[addr as usize & 0x3fff]
        }
    }
    fn prg_storeb(&mut self, _: u16, _: u8) {}  // Can't store to PRG-ROM.
    fn chr_loadb(&mut self, addr: u16) -> u8 { self.rom.chr[addr as usize] }
    fn chr_storeb(&mut self, _: u16, _: u8) {}  // Can't store to CHR-ROM.
    fn next_scanline(&mut self) -> MapperResult { MapperResult::Continue }
}

//
// Mapper 1 (SxROM/MMC1)
//
// See http://wiki.nesdev.com/w/index.php/Nintendo_MMC1
//

#[derive(Copy, Clone)]
struct SxCtrl{ val: u8 }

pub enum Mirroring {
    OneScreenLower,
    OneScreenUpper,
    Vertical,
    Horizontal,
}

enum SxPrgBankMode {
    /// Switch 32K at $8000, ignore low bit
    Switch32K,
    /// Fix first bank at $8000, switch 16K bank at $C000
    FixFirstBank,
    /// Fix last bank at $C000, switch 16K bank at $8000
    FixLastBank,
}

impl SxCtrl {
    fn prg_rom_mode(self) -> SxPrgBankMode {
        match (self.val >> 2) & 3 {
            0 | 1 => SxPrgBankMode::Switch32K,
            2 => SxPrgBankMode::FixFirstBank,
            3 => SxPrgBankMode::FixLastBank,
            _ => panic!("can't happen")
        }
    }
}

#[derive(Copy, Clone)]
struct SxRegs {
    /// $8000-$9FFF
    ctrl: SxCtrl,
    /// $A000-$BFFF
    chr_bank_0: u8,
    /// $C000-$DFFF
    chr_bank_1: u8,
    /// $E000-$FFFF
    prg_bank: u8,
}

pub struct SxRom {
    rom: Box<Rom>,
    regs: SxRegs,
    /// The internal accumulator.
    accum: u8,
    /// The write count. At the 5th write, we update the register.
    write_count: u8,
    //prg_ram: Box<[u8; 8192]>,
    chr_ram: Box<[u8; 8192]>,
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
            chr_ram: Box::new([ 0; 8192 ]),
        }
    }
}

impl Mapper for SxRom {
    fn prg_loadb(&mut self, addr: u16) -> u8 {
        if addr < 0x8000 {
            0u8
        } else if addr < 0xc000 {
            let bank = match self.regs.ctrl.prg_rom_mode() {
                SxPrgBankMode::Switch32K => self.regs.prg_bank & 0xfe,
                SxPrgBankMode::FixFirstBank => 0,
                SxPrgBankMode::FixLastBank => self.regs.prg_bank,
            };
            self.rom.prg[(bank as usize * 16384) | ((addr & 0x3fff) as usize)]
        } else {
            let bank = match self.regs.ctrl.prg_rom_mode() {
                SxPrgBankMode::Switch32K => (self.regs.prg_bank & 0xfe) | 1,
                SxPrgBankMode::FixFirstBank => self.regs.prg_bank,
                SxPrgBankMode::FixLastBank => (*self.rom).header.prg_rom_size - 1,
            };
            self.rom.prg[(bank as usize * 16384) | ((addr & 0x3fff) as usize)]
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
            self.regs.ctrl = SxCtrl{val: self.regs.ctrl.val | (3 << 2)};
            return;
        }

        // Write the lowest bit of the value into the right location of the accumulator.
        self.accum = self.accum | ((val & 1) << (self.write_count as usize));

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
    fn chr_loadb(&mut self, addr: u16) -> u8 {
        self.chr_ram[addr as usize]
    }

    fn chr_storeb(&mut self, addr: u16, val: u8) {
        self.chr_ram[addr as usize] = val
    }

    fn next_scanline(&mut self) -> MapperResult {
        MapperResult::Continue
    }
}

//
// Mapper 4 (TxROM/MMC3)
//
// See http://wiki.nesdev.com/w/index.php/MMC3
//

#[derive(Copy, Clone)]
struct TxBankSelect{ val: u8 }

impl Deref for TxBankSelect {
    type Target = u8;

    fn deref(&self) -> &u8 {
        &self.val
    }
}

enum TxPrgBankMode {
    Swappable8000,
    SwappableC000,
}

impl TxBankSelect {
    fn bank_update_select(&self) -> u8 {
        self.val & 0x7
    }

    fn prg_bank_mode(&self) -> TxPrgBankMode {
        if (self.val & 0x40) == 0 {
            TxPrgBankMode::Swappable8000
        } else {
            TxPrgBankMode::SwappableC000
        }
    }

    fn chr_a12_inversion(self) -> bool {
        (self.val & 0x80) != 0
    }
}

#[derive(Copy, Clone)]
struct TxRegs {
    bank_select: TxBankSelect,  // Bank select (0x8000-0x9ffe even)
}

struct TxRom {
    rom: Box<Rom>,
    regs: TxRegs,
    prg_ram: Box<[u8; 8192]>,

    chr_banks_2k: [u8; 2],    // 2KB CHR-ROM banks
    chr_banks_1k: [u8; 4],    // 1KB CHR-ROM banks
    prg_banks:    [u8; 2],    // 8KB PRG-ROM banks

    scanline_counter: u8,
    irq_reload: u8,             // Copied into the scanline counter when it hits zero.
    irq_enabled: bool,
}

impl TxRom {
    fn new(rom: Box<Rom>) -> TxRom {
        TxRom {
            rom: rom,
            regs: TxRegs { bank_select: TxBankSelect{val: 0} },
            prg_ram: Box::new([ 0; 8192 ]),

            chr_banks_2k: [ 0, 0 ],
            chr_banks_1k: [ 0, 0, 0, 0 ],
            prg_banks: [ 0, 0 ],

            scanline_counter: 0,
            irq_reload: 0,
            irq_enabled: false,
        }
    }

    fn prg_bank_count(&self) -> u8 {
        self.rom.header.prg_rom_size * 2
    }
}

impl Mapper for TxRom {
    fn prg_loadb(&mut self, addr: u16) -> u8 {
        if addr < 0x6000 {
            0u8
        } else if addr < 0x8000 {
            self.prg_ram[addr as usize & 0x1fff]
        } else if addr < 0xa000 {
            // $8000-$9FFF might be switchable or fixed to the second to last bank.
            let bank = match self.regs.bank_select.prg_bank_mode() {
                TxPrgBankMode::Swappable8000 => self.prg_banks[0],
                TxPrgBankMode::SwappableC000 => self.prg_bank_count() - 2,
            };
            self.rom.prg[(bank as usize * 8192) | (addr as usize & 0x1fff)]
        } else if addr < 0xc000 {
            // $A000-$BFFF is switchable.
            self.rom.prg[(self.prg_banks[1] as usize * 8192) | (addr as usize & 0x1fff)]
        } else if addr < 0xe000 {
            // $C000-$DFFF might be switchable or fixed to the second to last bank.
            let bank = match self.regs.bank_select.prg_bank_mode() {
                TxPrgBankMode::Swappable8000 => self.prg_bank_count() - 2,
                TxPrgBankMode::SwappableC000 => self.prg_banks[0],
            };
            self.rom.prg[(bank as usize * 8192) | (addr as usize & 0x1fff)]
        } else {
            // $E000-$FFFF is fixed to the last bank.
            let bank = self.prg_bank_count() - 1;
            self.rom.prg[(bank as usize * 8192) | (addr as usize & 0x1fff)]
        }
    }

    fn prg_storeb(&mut self, addr: u16, val: u8) {
        if addr < 0x6000 {
            return;
        }

        if addr < 0x8000 {
            self.prg_ram[addr as usize & 0x1fff] = val;
        } else if addr < 0xa000 {
            if (addr & 1) == 0 {
                // Bank select.
                self.regs.bank_select = TxBankSelect{val: val};
            } else {
                // Bank data.
                let bank_update_select = self.regs.bank_select.bank_update_select() as usize;
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

    fn chr_loadb(&mut self, addr: u16) -> u8 {
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
            self.rom.chr[(bank as usize * 1024) + (addr as usize & 0x7ff)]
        } else {
            self.rom.chr[(bank as usize * 1024) | (addr as usize & 0x3ff)]
        }
    }

    fn chr_storeb(&mut self, _: u16, _: u8) {
        // TODO: CHR-RAM
    }

    fn next_scanline(&mut self) -> MapperResult {
        if self.scanline_counter != 0 {
            self.scanline_counter -= 1;
            if self.scanline_counter == 0 {
                self.scanline_counter = self.irq_reload;

                if self.irq_enabled {
                    //debug!("*** Generated IRQ! ***");
                    return MapperResult::Irq;
                }
            }
        }
        MapperResult::Continue
    }
}
