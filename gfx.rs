//
// sprocketnes/gfx.rs
//
// Copyright (c) 2013 Mozilla Foundation
// Author: Patrick Walton
//

use ppu;

use core::uint::range;
use sdl::sdl::{InitTimer, InitVideo};
use sdl::sdl;
use sdl::video::{DoubleBuf, HWSurface, Surface};
use sdl::video;

const SCREEN_WIDTH: uint = 320;
const SCREEN_HEIGHT: uint = 240;

pub struct Gfx {
    screen: ~Surface
}

pub impl Gfx {
    static fn new() -> Gfx {
        sdl::init([ InitVideo, InitTimer ]);
        let screen = video::set_video_mode(SCREEN_WIDTH as int,
                                           SCREEN_HEIGHT as int,
                                           24,
                                           [ HWSurface ],
                                           [ DoubleBuf ]);
        Gfx { screen: screen.unwrap() }
    }

    fn blit(&self, ppu_screen: &([u8 * 184320])) {
        let src_stride = ppu::SCREEN_WIDTH * 3;
        let dest_stride = SCREEN_WIDTH * 3;

        do self.screen.with_lock |pixels| {
            for range(0, ppu::SCREEN_HEIGHT) |y| {
                let dest_start = y * dest_stride;
                let dest = vec::mut_view(pixels, dest_start, dest_start + dest_stride);
                let src_start = y * src_stride;
                let src = vec::view(*ppu_screen, src_start, src_start + src_stride);
                vec::bytes::copy_memory(dest, src, src_stride);
            }
        }
    }
}

