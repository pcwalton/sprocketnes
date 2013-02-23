//
// sprocketnes/rom.rs
//
// Author: Patrick Walton
//

use util::Fd;
use util;

use core::cast::transmute;
use core::libc::{O_RDONLY, c_int, size_t, ssize_t};
use core::libc;
use core::str;
use core::sys::size_of;
use core::vec;

pub struct Rom {
    header: INesHeader,
    prg: ~[u8],         // PRG-ROM
    chr: ~[u8],         // CHR-ROM
}

impl Rom {
    static fn from_fd(fd: Fd) -> Rom {
        let buffer = fd.read(size_of::<INesHeader>() as size_t);
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

        let prg_rom = fd.read(header.prg_rom_size as size_t * 16384);
        let chr_rom = if header.chr_rom_size > 0 {
            fd.read(header.chr_rom_size as size_t * 8192)
        } else {
            ~[]
        };
        Rom { header: header, prg: prg_rom, chr: chr_rom }
    }

    static fn from_path(path: &str) -> Rom {
        unsafe {
            do str::as_c_str(path) |c_path| {
                // FIXME: O_RDONLY should be a c_int in the first place!
                let fd = Fd(libc::open(c_path, O_RDONLY as c_int, 0));
                Rom::from_fd(fd)
            }
        }
    }
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
    fn mapper(&self) -> u8 { (self.flags_7 & 0xf0) | (self.flags_6 >> 4) }
    fn trainer(&self) -> bool { (self.flags_6 & 0x04) != 0 }

    fn to_str(&self) -> ~str {
        fmt!(
            "PRG-ROM size: %d\nCHR-ROM size: %d\nMapper: %d\nTrainer: %s",
            self.prg_rom_size as int,
            self.chr_rom_size as int,
            self.mapper() as int,
            if self.trainer() { "Yes" } else { "No" }
        )
    }
}

