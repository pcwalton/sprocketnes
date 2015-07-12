//! Contains ROM loading code (we use the iNES format).

//
// Author: Patrick Walton
//

use util;

use std::fs::File;
use std::fmt;
use std::path::Path;
use std::vec::Vec;

/// A ROM image
pub struct Rom {
    pub header: INesHeader,
    /// PRG-ROM
    pub prg: Vec<u8>,
    /// CHR-ROM
    pub chr: Vec<u8>,
}

impl Rom {
    fn from_file(file: &mut File) -> Rom {
        let mut buffer = [ 0; 16 ];
        util::read_to_buf(&mut buffer, file).unwrap();

        let header = INesHeader {
            magic: [
                buffer[0],
                buffer[1],
                buffer[2],
                buffer[3],
            ],
            prg_rom_size: buffer[4],
            chr_rom_size: buffer[5],
            flags_6: buffer[6],
            flags_7: buffer[7],
            prg_ram_size: buffer[8],
            flags_9: buffer[9],
            flags_10: buffer[10],
            zero: [ 0; 5 ]
        };

        assert!(header.magic == [
            'N' as u8,
            'E' as u8,
            'S' as u8,
            0x1a,
        ]);

        let mut prg_rom = vec![ 0u8; header.prg_rom_size as usize * 16384 ];
        util::read_to_buf(&mut prg_rom, file).unwrap();
        let mut chr_rom = vec![ 0u8; header.chr_rom_size as usize * 8192 ];
        util::read_to_buf(&mut chr_rom, file).unwrap();

        Rom {
            header: header,
            prg: prg_rom,
            chr: chr_rom,
        }
    }

    pub fn from_path(path: &Path) -> Rom {
        Rom::from_file(&mut File::open(path).unwrap())
    }
}

pub struct INesHeader {
    /// 'N' 'E' 'S' '\x1a'
    pub magic: [u8; 4],
    /// number of 16K units of PRG-ROM
    pub prg_rom_size: u8,
    /// number of 8K units of CHR-ROM
    pub chr_rom_size: u8,
    /// MMMMATPA
    ///
    /// * M: Low nibble of mapper number
    /// * A: 0xx0: vertical arrangement/horizontal mirroring (CIRAM A10 = PPU A11)
    ///      0xx1: horizontal arrangement/vertical mirroring (CIRAM A10 = PPU A10)
    ///      1xxx: four-screen VRAM
    /// * T: ROM contains a trainer
    /// * P: Cartridge has persistent memory
    pub flags_6: u8,
    /// MMMMVVPU
    ///
    /// * M: High nibble of mapper number
    /// * V: If 0b10, all following flags are in NES 2.0 format
    /// * P: ROM is for the PlayChoice-10
    /// * U: ROM is for VS Unisystem
    pub flags_7: u8,
    /// number of 8K units of PRG-RAM
    pub prg_ram_size: u8,
    /// RRRRRRRT
    ///
    /// * R: Reserved (= 0)
    /// * T: 0 for NTSC, 1 for PAL
    pub flags_9: u8,
    pub flags_10: u8,
    /// always zero
    pub zero: [u8; 5],
}

impl INesHeader {
    /// Returns the mapper ID.
    pub fn mapper(&self) -> u8 {
        (self.flags_7 & 0xf0) | (self.flags_6 >> 4)
    }

    /// Returns the low nibble of the mapper ID.
    pub fn ines_mapper(&self) -> u8 {
        self.flags_6 >> 4
    }

    pub fn trainer(&self) -> bool {
        (self.flags_6 & 0x04) != 0
    }
}

impl fmt::Display for INesHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "PRG-ROM: {} KB, CHR-ROM: {} KB, Mapper: {} ({}), Trainer: {}",
            self.prg_rom_size as u16 * 16,
            self.chr_rom_size as u16 * 8,
            self.mapper() as isize,
            self.ines_mapper() as isize,
            self.trainer(),
        )
    }
}
