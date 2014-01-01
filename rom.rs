//
// sprocketnes/rom.rs
//
// Author: Patrick Walton
//

use std::io::File;
use std::vec;

pub struct Rom {
    header: INesHeader,
    prg: ~[u8],         // PRG-ROM
    chr: ~[u8],         // CHR-ROM
}

impl Rom {
    fn from_file(file: &mut File) -> Rom {
        let mut buffer = [ 0, ..16 ];
        file.read(buffer);

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
            zero: [ 0, ..5 ]
        };

        assert!(header.magic == [
            'N' as u8,
            'E' as u8,
            'S' as u8,
            0x1a,
        ]);

        let mut prg_rom = vec::from_elem(header.prg_rom_size as uint * 16384, 0u8);
        file.read(prg_rom);
        let mut chr_rom = vec::from_elem(header.chr_rom_size as uint * 8192, 0u8);
        file.read(chr_rom);

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

struct INesHeader {
    magic: [u8, ..4],   // 'N' 'E' 'S' '\x1a'
    prg_rom_size: u8,   // number of 16K units of PRG-ROM
    chr_rom_size: u8,   // number of 8K units of CHR-ROM
    flags_6: u8,
    flags_7: u8,
    prg_ram_size: u8,   // number of 8K units of PRG-RAM
    flags_9: u8,
    flags_10: u8,
    zero: [u8, ..5],    // always zero
}

impl INesHeader {
    pub fn mapper(&self) -> u8 {
        (self.flags_7 & 0xf0) | (self.flags_6 >> 4)
    }
    pub fn ines_mapper(&self) -> u8 {
        self.flags_6 >> 4
    }
    pub fn trainer(&self) -> bool {
        (self.flags_6 & 0x04) != 0
    }

    pub fn to_str(&self) -> ~str {
        format!("PRG-ROM size: {}\nCHR-ROM size: {}\nMapper: {}/{}\nTrainer: {}",
                self.prg_rom_size as int,
                self.chr_rom_size as int,
                self.mapper() as int,
                self.ines_mapper() as int,
                if self.trainer() {
                    "Yes"
                } else {
                    "No"
                })
    }
}

