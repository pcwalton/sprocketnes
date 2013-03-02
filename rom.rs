//
// sprocketnes/rom.rs
//
// Author: Patrick Walton
//

use util::{Fd, ForReading};

use core::vec;

pub struct Rom {
    header: INesHeader,
    prg: ~[u8],         // PRG-ROM
    chr: ~[u8],         // CHR-ROM
}

impl Rom {
    static fn from_fd(fd: Fd) -> Rom {
        let mut buffer = [ 0, ..16 ];
        fd.read(buffer);

        let header = INesHeader {
            magic: [ buffer[0], buffer[1], buffer[2], buffer[3] ],
            prg_rom_size: buffer[4],
            chr_rom_size: buffer[5],
            flags_6: buffer[6],
            flags_7: buffer[7],
            prg_ram_size: buffer[8],
            flags_9: buffer[9],
            flags_10: buffer[10],
            zero: [ 0, ..5 ]
        };

        assert header.magic == [ 'N' as u8, 'E' as u8, 'S' as u8, 0x1a ];

        let mut prg_rom = vec::from_elem(header.prg_rom_size as uint * 16384, 0);
        fd.read(prg_rom);
        let mut chr_rom = vec::from_elem(header.chr_rom_size as uint * 8192, 0);
        fd.read(chr_rom);

        Rom { header: header, prg: prg_rom, chr: chr_rom }
    }

    static fn from_path(path: &str) -> Rom { Rom::from_fd(Fd::open(path, ForReading)) }
}

struct INesHeader {
    magic: [u8 * 4],    // 'N' 'E' 'S' '\x1a'
    prg_rom_size: u8,   // number of 16K units of PRG-ROM
    chr_rom_size: u8,   // number of 8K units of CHR-ROM
    flags_6: u8,
    flags_7: u8,
    prg_ram_size: u8,   // number of 8K units of PRG-RAM
    flags_9: u8,
    flags_10: u8,
    zero: [u8 * 5],     // always zero
}

impl INesHeader {
    pub fn mapper(&self) -> u8 { (self.flags_7 & 0xf0) | (self.flags_6 >> 4) }
    pub fn trainer(&self) -> bool { (self.flags_6 & 0x04) != 0 }

    pub fn to_str(&self) -> ~str {
        fmt!(
            "PRG-ROM size: %d\nCHR-ROM size: %d\nMapper: %d\nTrainer: %s",
            self.prg_rom_size as int,
            self.chr_rom_size as int,
            self.mapper() as int,
            if self.trainer() { "Yes" } else { "No" }
        )
    }
}

