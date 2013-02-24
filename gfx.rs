//
// sprocketnes/gfx.rs
//
// Author: Patrick Walton
//

use core::cast::transmute;
use core::uint::range;
use sdl::sdl::{InitAudio, InitTimer, InitVideo};
use sdl::sdl;
use sdl::video::{AsyncBlit, DoubleBuf, SWSurface, Surface};
use sdl::video;

const SCREEN_WIDTH: uint = 256;
const SCREEN_HEIGHT: uint = 240;

pub enum Scale {
    Scale1x,
    Scale3x,
}

impl Scale {
    fn factor(self) -> uint { match self { Scale1x => 1, Scale3x => 3 } }
}

pub struct Gfx {
    screen: ~Surface,
    scale: Scale,
}

impl Gfx {
    static fn new(scale: Scale) -> Gfx {
        sdl::init([ InitVideo, InitAudio, InitTimer ]);
        let screen = video::set_video_mode(SCREEN_WIDTH * scale.factor() as int,
                                           SCREEN_HEIGHT * scale.factor()  as int,
                                           24,
                                           [ SWSurface, AsyncBlit ],
                                           [ DoubleBuf ]);

        Gfx { screen: screen.unwrap(), scale: scale }
    }

    fn blit(&self, ppu_screen: &([u8 * 184320])) {
        do self.screen.with_lock |pixels| {
            match self.scale {
                Scale1x => vec::bytes::copy_memory(pixels, *ppu_screen, ppu_screen.len()),
                Scale3x => {
                    // Ugh, LLVM isn't inlining properly.
                    //
                    // FIXME: Mark for loop bodies as always inline.

                    let mut dest = 0;
                    let mut src_y = 0;
                    while src_y < SCREEN_HEIGHT {
                        let src_scanline_offset = src_y * SCREEN_WIDTH * 3;
                        let mut repeat = 0;
                        while repeat < 3 {
                            let mut src_x = 0;
                            while src_x < SCREEN_WIDTH {
                                // TODO: Unaligned 64-bit write for speed.
                                let r = ppu_screen[src_scanline_offset + src_x*3 + 0];
                                let g = ppu_screen[src_scanline_offset + src_x*3 + 1];
                                let b = ppu_screen[src_scanline_offset + src_x*3 + 2];
                                pixels[dest + 0] = r;
                                pixels[dest + 1] = g;
                                pixels[dest + 2] = b;
                                pixels[dest + 3] = r;
                                pixels[dest + 4] = g;
                                pixels[dest + 5] = b;
                                pixels[dest + 6] = r;
                                pixels[dest + 7] = g;
                                pixels[dest + 8] = b;
                                dest += 9;

                                src_x += 1;
                            }
                            repeat += 1;
                        }
                        src_y += 1;
                    }
                }
            }
        }
    }
}

