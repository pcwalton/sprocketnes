//
// sprocketnes/rom.rs
//
// Author: Patrick Walton
//

use std::io::File;
use std::vec::Vec;

pub struct Rom {
    pub header: INesHeader,
    pub prg: Vec<u8>,         // PRG-ROM
    pub chr: Vec<u8>,         // CHR-ROM
}

impl Rom {
    fn from_file(file: &mut File) -> Rom {
        let mut buffer = [ 0, ..16 ];
        file.fill(buffer).unwrap();

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

        let mut prg_rom = Vec::from_elem(header.prg_rom_size as uint * 16384, 0u8);
        file.fill(prg_rom.as_mut_slice()).unwrap();
        let mut chr_rom = Vec::from_elem(header.chr_rom_size as uint * 8192, 0u8);
        file.fill(chr_rom.as_mut_slice()).unwrap();

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
    pub magic: [u8, ..4],   // 'N' 'E' 'S' '\x1a'
    pub prg_rom_size: u8,   // number of 16K units of PRG-ROM
    pub chr_rom_size: u8,   // number of 8K units of CHR-ROM
    pub flags_6: u8,
    pub flags_7: u8,
    pub prg_ram_size: u8,   // number of 8K units of PRG-RAM
    pub flags_9: u8,
    pub flags_10: u8,
    pub zero: [u8, ..5],    // always zero
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

