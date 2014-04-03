//
// sprocketnes/gfx.rs
//
// Author: Patrick Walton
//

use sdl::sdl::{InitAudio, InitTimer, InitVideo};
use sdl::sdl;
use sdl::video::{SWSurface, Surface};
use sdl::video;
use std::cast::transmute;

static SCREEN_WIDTH: uint = 256;
static SCREEN_HEIGHT: uint = 240;

static FONT_HEIGHT: uint = 10;
static FONT_GLYPH_COUNT: uint = 95;
static FONT_GLYPH_LENGTH: uint = FONT_GLYPH_COUNT * FONT_HEIGHT;

static STATUS_LINE_PADDING: uint = 6;
static STATUS_LINE_X: uint = STATUS_LINE_PADDING;
static STATUS_LINE_Y: uint = SCREEN_HEIGHT - STATUS_LINE_PADDING - FONT_HEIGHT;
static STATUS_LINE_PAUSE_DURATION: uint = 120;                   // in 1/60 of a second

//
// PT Ronda Seven
//
// (c) Yusuke Kamiyamane, http://pinvoke.com/
//

static FONT_GLYPHS: [u8, ..FONT_GLYPH_LENGTH] = [
      0,   0,   0,   0,   0,   0,   0,   0,   0,   0,  // ' '
      0,  64,  64,  64,  64,  64,   0,  64,   0,   0,  // '!'
      0, 160, 160,   0,   0,   0,   0,   0,   0,   0,  // '"'
      0,  80,  80, 248,  80, 248,  80,  80,   0,   0,  // '#'
     32, 112, 168, 160, 112,  40, 168, 112,  32,   0,  // '$'
      0,  66, 164,  72,  16,  36,  74, 132,   0,   0,  // '%'
      0,  96, 144, 160,  72, 168, 144, 104,   0,   0,  // '&'
      0, 128, 128,   0,   0,   0,   0,   0,   0,   0,  // '''
     32,  64, 128, 128, 128, 128, 128,  64,  32,   0,  // '('
    128,  64,  32,  32,  32,  32,  32,  64, 128,   0,  // ')'
      0,  32, 168, 112, 168,  32,   0,   0,   0,   0,  // '*'
      0,   0,  32,  32, 248,  32,  32,   0,   0,   0,  // '+'
      0,   0,   0,   0,   0,   0,   0,  64,  64, 128,  // ','
      0,   0,   0,   0,   0, 224,   0,   0,   0,   0,  // '-'
      0,   0,   0,   0,   0,   0,   0,  64,   0,   0,  // '.'
      8,   8,  16,  16,  32,  64,  64, 128, 128,   0,  // '/'
      0, 112, 136, 136, 136, 136, 136, 112,   0,   0,  // '0'
      0, 192,  64,  64,  64,  64,  64,  64,   0,   0,  // '1'
      0, 112, 136,   8,  16,  32,  64, 248,   0,   0,  // '2'
      0, 112, 136,   8,  48,   8, 136, 112,   0,   0,  // '3'
      0,  48,  80,  80, 144, 248,  16,  16,   0,   0,  // '4'
      0, 248, 128, 128, 240,   8, 136, 112,   0,   0,  // '5'
      0, 112, 136, 128, 240, 136, 136, 112,   0,   0,  // '6'
      0, 248,   8,  16,  16,  32,  32,  64,   0,   0,  // '7'
      0, 112, 136, 136, 112, 136, 136, 112,   0,   0,  // '8'
      0, 112, 136, 136, 120,   8, 136, 112,   0,   0,  // '9'
      0,   0,   0,  64,   0,   0,   0,  64,   0,   0,  // ':'
      0,   0,   0,  64,   0,   0,   0,  64,  64, 128,  // ';'
      0,   0,  32,  64, 128,  64,  32,   0,   0,   0,  // '<'
      0,   0,   0, 224,   0, 224,   0,   0,   0,   0,  // '='
      0,   0, 128,  64,  32,  64, 128,   0,   0,   0,  // '>'
      0, 112, 136,   8,  16,  32,   0,  32,   0,   0,  // '?'
     60,  66, 157, 165, 165, 173, 149,  66,  56,   0,  // '@'
      0, 112, 136, 136, 248, 136, 136, 136,   0,   0,  // 'A'
      0, 240, 136, 136, 240, 136, 136, 240,   0,   0,  // 'B'
      0, 112, 136, 128, 128, 128, 136, 112,   0,   0,  // 'C'
      0, 240, 136, 136, 136, 136, 136, 240,   0,   0,  // 'D'
      0, 248, 128, 128, 240, 128, 128, 248,   0,   0,  // 'E'
      0, 248, 128, 128, 240, 128, 128, 128,   0,   0,  // 'F'
      0, 112, 136, 128, 184, 136, 152, 104,   0,   0,  // 'G'
      0, 136, 136, 136, 248, 136, 136, 136,   0,   0,  // 'H'
      0, 128, 128, 128, 128, 128, 128, 128,   0,   0,  // 'I'
      0,  16,  16,  16,  16,  16, 144,  96,   0,   0,  // 'J'
      0, 136, 144, 160, 192, 160, 144, 136,   0,   0,  // 'K'
      0, 128, 128, 128, 128, 128, 128, 240,   0,   0,  // 'L'
      0, 130, 198, 170, 146, 130, 130, 130,   0,   0,  // 'M'
      0, 136, 200, 168, 168, 168, 152, 136,   0,   0,  // 'N'
      0, 112, 136, 136, 136, 136, 136, 112,   0,   0,  // 'O'
      0, 240, 136, 136, 240, 128, 128, 128,   0,   0,  // 'P'
      0, 112, 136, 136, 136, 136, 136, 112,   8,   0,  // 'Q'
      0, 240, 136, 136, 240, 160, 144, 136,   0,   0,  // 'R'
      0, 112, 136, 128, 112,   8, 136, 112,   0,   0,  // 'S'
      0, 248,  32,  32,  32,  32,  32,  32,   0,   0,  // 'T'
      0, 136, 136, 136, 136, 136, 136, 112,   0,   0,  // 'U'
      0, 136, 136,  80,  80,  80,  32,  32,   0,   0,  // 'V'
      0, 146, 146, 146, 146, 146, 146, 108,   0,   0,  // 'W'
      0, 136, 136,  80,  32,  80, 136, 136,   0,   0,  // 'X'
      0, 136, 136,  80,  32,  32,  32,  32,   0,   0,  // 'Y'
      0, 248,   8,  16,  32,  64, 128, 248,   0,   0,  // 'Z'
    224, 128, 128, 128, 128, 128, 128, 128, 224,   0,  // '['
    128, 128,  64,  64,  32,  16,  16,   8,   8,   0,  // '\'
    224,  32,  32,  32,  32,  32,  32,  32, 224,   0,  // ']'
      0,  64, 160,   0,   0,   0,   0,   0,   0,   0,  // '^'
      0,   0,   0,   0,   0,   0,   0, 224,   0,   0,  // '_'
      0,   0,   0,   0,   0,   0,   0,   0,   0,   0,  // '`'
      0,   0,   0, 112, 144, 144, 176,  80,   0,   0,  // 'a'
      0, 128, 128, 160, 208, 144, 144, 224,   0,   0,  // 'b'
      0,   0,   0,  96, 144, 128, 144,  96,   0,   0,  // 'c'
      0,  16,  16, 112, 144, 144, 176,  80,   0,   0,  // 'd'
      0,   0,   0,  96, 144, 240, 128,  96,   0,   0,  // 'e'
      0,  96, 128, 192, 128, 128, 128, 128,   0,   0,  // 'f'
      0,   0,   0, 112, 144, 144, 176,  80,  16,  96,  // 'g'
      0, 128, 128, 160, 208, 144, 144, 144,   0,   0,  // 'h'
      0, 128,   0, 128, 128, 128, 128, 128,   0,   0,  // 'i'
      0, 128,   0, 128, 128, 128, 128, 128, 128,   0,  // 'j'
      0, 128, 128, 144, 160, 192, 160, 144,   0,   0,  // 'k'
      0, 128, 128, 128, 128, 128, 128, 128,   0,   0,  // 'l'
      0,   0,   0, 164, 218, 146, 146, 146,   0,   0,  // 'm'
      0,   0,   0, 160, 208, 144, 144, 144,   0,   0,  // 'n'
      0,   0,   0,  96, 144, 144, 144,  96,   0,   0,  // 'o'
      0,   0,   0, 160, 208, 144, 144, 224, 128, 128,  // 'p'
      0,   0,   0, 112, 144, 144, 176,  80,  16,  16,  // 'q'
      0,   0,   0, 160, 192, 128, 128, 128,   0,   0,  // 'r'
      0,   0,   0,  96, 128,  64,  32, 192,   0,   0,  // 's'
      0,  64,  64, 224,  64,  64,  64,  64,   0,   0,  // 't'
      0,   0,   0, 144, 144, 144, 176,  80,   0,   0,  // 'u'
      0,   0,   0, 144, 144, 144, 144,  96,   0,   0,  // 'v'
      0,   0,   0, 146, 146, 146, 146, 108,   0,   0,  // 'w'
      0,   0,   0, 144, 144,  96, 144, 144,   0,   0,  // 'x'
      0,   0,   0, 144, 144, 144, 176,  80,  16,  96,  // 'y'
      0,   0,   0, 224,  32,  64, 128, 224,   0,   0,  // 'z'
     32,  64,  64,  64, 128,  64,  64,  64,  32,   0,  // '{'
    128, 128, 128, 128, 128, 128, 128, 128, 128,   0,  // '|'
    128,  64,  64,  64,  32,  64,  64,  64, 128,   0,  // '}'
      0,  80, 160,   0,   0,   0,   0,   0,   0,   0,  // '~'
];

static FONT_ADVANCES: [u8, ..FONT_GLYPH_COUNT] = [
    3 /*   */, 3 /* ! */, 4 /* " */, 6 /* # */, 6 /* $ */, 8 /* % */, 6 /* & */, 2 /* ' */, 
    4 /* ( */, 4 /* ) */, 6 /* * */, 6 /* + */, 3 /* , */, 4 /* - */, 3 /* . */, 5 /* / */, 
    6 /* 0 */, 3 /* 1 */, 6 /* 2 */, 6 /* 3 */, 6 /* 4 */, 6 /* 5 */, 6 /* 6 */, 6 /* 7 */, 
    6 /* 8 */, 6 /* 9 */, 3 /* : */, 3 /* ; */, 4 /* < */, 4 /* = */, 4 /* > */, 6 /* ? */, 
    8 /* @ */, 6 /* A */, 6 /* B */, 6 /* C */, 6 /* D */, 6 /* E */, 6 /* F */, 6 /* G */, 
    6 /* H */, 2 /* I */, 5 /* J */, 6 /* K */, 5 /* L */, 8 /* M */, 6 /* N */, 6 /* O */, 
    6 /* P */, 6 /* Q */, 6 /* R */, 6 /* S */, 6 /* T */, 6 /* U */, 6 /* V */, 8 /* W */, 
    6 /* X */, 6 /* Y */, 6 /* Z */, 4 /* [ */, 6 /* \ */, 4 /* ] */, 4 /* ^ */, 4 /* _ */, 
    3 /* ` */, 5 /* a */, 5 /* b */, 5 /* c */, 5 /* d */, 5 /* e */, 3 /* f */, 5 /* g */, 
    5 /* h */, 2 /* i */, 2 /* j */, 5 /* k */, 2 /* l */, 8 /* m */, 5 /* n */, 5 /* o */, 
    5 /* p */, 5 /* q */, 4 /* r */, 4 /* s */, 4 /* t */, 5 /* u */, 5 /* v */, 8 /* w */, 
    5 /* x */, 5 /* y */, 4 /* z */, 4 /* { */, 2 /* | */, 4 /* } */, 5 /* ~ */, 
];

//
// Text output
//

enum GlyphColor {
    White,
    Black,
}

fn draw_glyph(pixels: &mut [u8],
              surface_width: uint,
              x: int,
              y: int,
              color: GlyphColor,
              glyph_index: uint) {
    let color_byte = match color {
        White => 0xff,
        Black => 0x00,
    };
    for y_index in range(0, 10) {
        let row = FONT_GLYPHS[glyph_index * 10 + y_index as uint];
        for x_index in range(0, 8) {
            if ((row >> (7 - x_index)) & 1) != 0 {
                for channel in range(0, 3) {
                    let mut index = (y + y_index) * (surface_width as int) * 3 + (x + x_index) * 3;
                    index += channel;

                    if index >= 0 && index < pixels.len() as int {
                        pixels[index as uint] = color_byte;
                    }
                }
            }
        }
    }
}

pub fn draw_text(pixels: &mut [u8], surface_width: uint, mut x: int, y: int, string: &str) {
    for i in range(0u, string.len()) {
        let glyph_index = (string[i] - 32) as uint;
        if glyph_index < FONT_ADVANCES.len() {
            draw_glyph(pixels, surface_width, x, y + 1, Black, glyph_index);    // Shadow
            draw_glyph(pixels, surface_width, x, y, White, glyph_index);        // Main
            x += FONT_ADVANCES[glyph_index] as int;
        }
    }
}

#[deriving(Eq)]
enum StatusLineAnimation {
    Idle,
    Pausing(uint),
    SlidingOut(uint),
}

struct StatusLineText {
    string: ~str,
    animation: StatusLineAnimation,
}

impl StatusLineText {
    fn new() -> StatusLineText {
        StatusLineText {
            string: "".to_str(),
            animation: Idle,
        }
    }

    fn set(&mut self, string: ~str) {
        self.string = string;
        self.animation = Pausing(STATUS_LINE_PAUSE_DURATION);
    }

    fn tick(&mut self) {
        self.animation = match self.animation {
            Idle                      => Idle,
            Pausing(0)                => SlidingOut(STATUS_LINE_Y),
            Pausing(time)             => Pausing(time - 1),
            SlidingOut(SCREEN_HEIGHT) => Idle,
            SlidingOut(y)             => SlidingOut(y + 1),
        }
    }

    fn render(&self, pixels: &mut [u8]) {
        if self.animation == Idle {
            return;
        }
        let y = match self.animation {
            Idle => fail!(),
            SlidingOut(y) => y as int,
            Pausing(_) => STATUS_LINE_Y as int,
        };
        draw_text(pixels, SCREEN_WIDTH, STATUS_LINE_X as int, y, self.string);
    }
}

pub struct StatusLine {
    pub text: StatusLineText,
}

impl StatusLine {
    pub fn new() -> StatusLine       { StatusLine { text: StatusLineText::new() } }
    pub fn set(&mut self, new_text: ~str)   { self.text.set(new_text);                   }
    pub fn render(&self, pixels: &mut [u8]) { self.text.render(pixels);                  }
}

//
// Screen scaling
//

pub enum Scale {
    Scale1x,
    Scale2x,
    Scale3x,
}

impl Scale {
    fn factor(self) -> uint { match self { Scale1x => 1, Scale2x => 2, Scale3x => 3 } }
}

pub struct Gfx {
    pub screen: ~Surface,
    pub scale: Scale,
    pub status_line: StatusLine,
}

macro_rules! scaler(
    ($count:expr, $pixels:ident, $ppu_screen:ident) => (
        // Type safety goes out the window when we're in a performance critical loop
        // like this!

        unsafe {
            let mut dest: *mut u32 = transmute(&$pixels[0]);
            let src_start: *u8 = transmute(&$ppu_screen[0]);

            let mut src_y = 0;
            while src_y < SCREEN_HEIGHT {
                let src_scanline_start = src_start.offset((src_y * SCREEN_WIDTH * 3) as int);
                let src_scanline_end = src_scanline_start.offset((SCREEN_WIDTH * 3) as int);

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

//
// Main graphics routine
//

impl Gfx {
    pub fn new(scale: Scale) -> Gfx {
        sdl::init([ InitVideo, InitAudio, InitTimer ]);
        let screen = video::set_video_mode((SCREEN_WIDTH * scale.factor()) as int,
                                           (SCREEN_HEIGHT * scale.factor()) as int,
                                           32,
                                           [ SWSurface ],
                                           []);

        Gfx { screen: screen.unwrap(), scale: scale, status_line: StatusLine::new() }
    }

    pub fn tick(&mut self) {
        self.status_line.text.tick();
    }

    pub fn composite(&self, ppu_screen: &mut ([u8, ..184320])) {
        self.status_line.render(*ppu_screen);
        self.blit(ppu_screen);
    }

    fn blit(&self, ppu_screen: &([u8, ..184320])) {
        self.screen.with_lock(|pixels| {
            match self.scale {
                Scale1x => scaler!(1, pixels, ppu_screen),
                Scale2x => scaler!(2, pixels, ppu_screen),
                Scale3x => scaler!(3, pixels, ppu_screen),
            }
        })
    }
}

