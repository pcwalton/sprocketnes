//
// sprocketnes/gfx.rs
//
// Author: Patrick Walton
//

use core::cast::transmute;
use core::uint::range;
use sdl::sdl::{InitAudio, InitTimer, InitVideo};
use sdl::sdl;
use sdl::video::{AsyncBlit, SWSurface, Surface};
use sdl::video;

const SCREEN_WIDTH: uint = 256;
const SCREEN_HEIGHT: uint = 240;

pub enum Scale {
    Scale1x,
    Scale2x,
    Scale3x,
}

impl Scale {
    fn factor(self) -> uint { match self { Scale1x => 1, Scale2x => 2, Scale3x => 3 } }
}

pub struct Gfx {
    screen: ~Surface,
    scale: Scale,
}

macro_rules! scaler(
    ($count:expr, $pixels:ident, $ppu_screen:ident) => (
        // Type safety goes out the window when we're in a performance critical loop
        // like this!

        unsafe {
            let mut dest: *mut u32 = transmute(&$pixels[0]);
            let mut src_start: *u8 = transmute(&$ppu_screen[0]);

            let mut src_y = 0;
            while src_y < SCREEN_HEIGHT {
                let src_scanline_start = src_start.offset(src_y * SCREEN_WIDTH * 3);
                let src_scanline_end = src_scanline_start.offset(SCREEN_WIDTH * 3);

                // Ugh, LLVM isn't inlining properly.
                //
                // FIXME: Mark for loop bodies as always inline in rustc.
                let mut repeat_y = 0;
                while repeat_y < $count {
                    let mut src = src_scanline_start;
                    while src < src_scanline_end {
                        let r = *src as u32; src = src.offset(1);
                        let g = *src as u32; src = src.offset(1);
                        let b = *src as u32; src = src.offset(1);
                        let pixel = (r << 24) | (g << 16) | (b << 8);

                        let mut repeat_x = 0;
                        while repeat_x < $count { 
                            *dest = pixel; dest = dest.offset(1);
                            repeat_x += 1;
                        }
                    }
                    repeat_y += 1;
                }
                src_y += 1;
            }
        }
    )
)

impl Gfx {
    static pub fn new(scale: Scale) -> Gfx {
        sdl::init([ InitVideo, InitAudio, InitTimer ]);
        let screen = video::set_video_mode(SCREEN_WIDTH * scale.factor() as int,
                                           SCREEN_HEIGHT * scale.factor()  as int,
                                           32,
                                           [ SWSurface ],
                                           []);

        Gfx { screen: screen.unwrap(), scale: scale }
    }

    pub fn blit(&self, ppu_screen: &([u8 * 184320])) {
        do self.screen.with_lock |pixels| {
            match self.scale {
                Scale1x => scaler!(1, pixels, ppu_screen),
                Scale2x => scaler!(2, pixels, ppu_screen),
                Scale3x => scaler!(3, pixels, ppu_screen),
            }
        }
    }
}

