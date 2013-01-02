//
// sprocketnes/rom.rs
//
// Copyright (c) 2012 Mozilla Foundation
// Author: Patrick Walton
//

use core::cast::transmute;
use core::libc::{O_RDONLY, c_int, size_t, ssize_t};
use core::libc;
use core::sys::size_of;
use core::vec;

// Blech! This really should go in the standard library!
struct Fd(c_int);
impl Fd : Drop {
    fn finalize(&self) { libc::close(**self); }
}

pub struct Rom {
    header: INesHeader,
    prg: ~[u8],         // PRG-ROM
    chr: ~[u8],         // CHR-ROM
}

// FIXME: `pub` should not be required here! Sigh. Resolve bug.
pub impl Rom {
    static fn from_fd(fd: c_int) -> Rom {
        // Unsafe, I know... but I want to read into a POD struct and Rust can't prove that safe
        // right now.
        unsafe {
            let mut header = INesHeader {
                magic: [ 0, ..4 ],
                prg_rom_size: 0,
                chr_rom_size: 0,
                flags_6: 0,
                flags_7: 0,
                prg_ram_size: 0,
                flags_9: 0,
                flags_10: 0,
                zero: [ 0, ..5 ]
            };
            let sz = size_of::<INesHeader>() as size_t;
            assert libc::read(fd, transmute(&mut header), sz) as size_t == sz;

            assert header.magic == [ 'N' as u8, 'E' as u8, 'S' as u8, 0x1a ];

            let read: &fn(sz: size_t) -> ~[u8] = |sz| {
                let mut result = vec::from_elem(sz as uint, 0);
                assert libc::read(fd, transmute(&mut result[0]), sz) as size_t == sz;
                result
            };

            let prg_rom = read(header.prg_rom_size as size_t * 16384);
            let chr_rom = read(header.chr_rom_size as size_t * 8192);
            Rom { header: header, prg: prg_rom, chr: chr_rom }
        }
    }

    static fn from_path(path: &str) -> Rom {
        do str::as_c_str(path) |c_path| {
            // FIXME: O_RDONLY should be a c_int in the first place!
            let fd = Fd(libc::open(c_path, O_RDONLY as c_int, 0));
            Rom::from_fd(*fd)
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

