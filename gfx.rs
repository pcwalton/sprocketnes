//
// sprocketnes/gfx.rs
//
// Author: Patrick Walton
//

use sdl2::{INIT_AUDIO, INIT_TIMER, INIT_VIDEO, INIT_EVENTS};
use sdl2::pixels::BGR24;
use sdl2::rect::Rect;
use sdl2::render::{ACCELERATED, Renderer, RenderDriverIndex, Texture, TextureAccess};
use sdl2::video::{PosCentered, Window, INPUT_FOCUS};
use sdl2;

use libc::{int32_t, uint8_t};

const SCREEN_WIDTH: uint = 256;
const SCREEN_HEIGHT: uint = 240;

const FONT_HEIGHT: uint = 10;
const FONT_GLYPH_COUNT: uint = 95;
const FONT_GLYPH_LENGTH: uint = FONT_GLYPH_COUNT * FONT_HEIGHT;

const STATUS_LINE_PADDING: uint = 6;
const STATUS_LINE_X: uint = STATUS_LINE_PADDING;
const STATUS_LINE_Y: uint = SCREEN_HEIGHT - STATUS_LINE_PADDING - FONT_HEIGHT;
const STATUS_LINE_PAUSE_DURATION: uint = 120;                   // in 1/60 of a second

#[allow(dead_code)]
const SCREEN_SIZE: uint = 184320;

//
// PT Ronda Seven
//
// (c) Yusuke Kamiyamane, http://pinvoke.com/
//

const FONT_GLYPHS: [uint8_t, ..FONT_GLYPH_LENGTH] = [
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

const FONT_ADVANCES: [uint8_t, ..FONT_GLYPH_COUNT] = [
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

fn draw_glyph(pixels: &mut [uint8_t],
              surface_width: uint,
              x: int,
              y: int,
              color: GlyphColor,
              glyph_index: uint) {
    let color_byte = match color {
        GlyphColor::White => 0xff,
        GlyphColor::Black => 0x00,
    };
    for y_index in range(0, 10) {
        let row = FONT_GLYPHS[glyph_index * 10 + y_index as uint];
        for x_index in range(0, 8) {
            if ((row >> (7 - x_index) as uint) & 1) != 0 {
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

pub fn draw_text(pixels: &mut [uint8_t], surface_width: uint, mut x: int, y: int, string: &str) {
    for i in range(0u, string.len()) {
        let glyph_index = (string.as_bytes()[i] - 32) as uint;
        if glyph_index < FONT_ADVANCES.len() {
            draw_glyph(pixels, surface_width, x, y + 1, GlyphColor::Black, glyph_index);    // Shadow
            draw_glyph(pixels, surface_width, x, y, GlyphColor::White, glyph_index);        // Main
            x += FONT_ADVANCES[glyph_index] as int;
        }
    }
}

#[deriving(PartialEq, Eq)]
enum StatusLineAnimation {
    Idle,
    Pausing(uint),
    SlidingOut(uint),
}

struct StatusLineText {
    string: String,
    animation: StatusLineAnimation,
}

impl StatusLineText {
    fn new() -> StatusLineText {
        StatusLineText {
            string: "".to_string(),
            animation: StatusLineAnimation::Idle,
        }
    }

    fn set(&mut self, string: String) {
        self.string = string;
        self.animation = StatusLineAnimation::Pausing(STATUS_LINE_PAUSE_DURATION);
    }

    fn tick(&mut self) {
        use self::StatusLineAnimation::{Idle, Pausing, SlidingOut};
        self.animation = match self.animation {
            Idle                      => Idle,
            Pausing(0)                => SlidingOut(STATUS_LINE_Y),
            Pausing(time)             => Pausing(time - 1),
            SlidingOut(SCREEN_HEIGHT) => Idle,
            SlidingOut(y)             => SlidingOut(y + 1),
        }
    }

    fn render(&self, pixels: &mut [uint8_t]) {
        if self.animation == StatusLineAnimation::Idle {
            return;
        }
        let y = match self.animation {
            StatusLineAnimation::Idle => panic!(),
            StatusLineAnimation::SlidingOut(y) => y as int,
            StatusLineAnimation::Pausing(_) => STATUS_LINE_Y as int,
        };
        draw_text(pixels, SCREEN_WIDTH, STATUS_LINE_X as int, y, self.string.as_slice());
    }
}

pub struct StatusLine {
    text: StatusLineText,
}

impl StatusLine {
    pub fn new() -> StatusLine {
        StatusLine {
            text: StatusLineText::new(),
        }
    }
    pub fn set(&mut self, new_text: String) {
        self.text.set(new_text);
    }
    pub fn render(&self, pixels: &mut [uint8_t]) {
        self.text.render(pixels);
    }
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
    fn factor(self) -> uint {
        match self {
            Scale::Scale1x => 1,
            Scale::Scale2x => 2,
            Scale::Scale3x => 3,
        }
    }
}

pub struct Gfx {
    pub renderer: Box<Renderer>,
    pub texture: Box<Texture>,
    pub scale: Scale,
    pub status_line: StatusLine,
}

//
// Main graphics routine
//

impl Gfx {
    pub fn new(scale: Scale) -> Gfx {
        sdl2::init(INIT_VIDEO | INIT_AUDIO | INIT_TIMER | INIT_EVENTS);
        let window = Window::new("sprocketnes",
                                 PosCentered,
                                 PosCentered,
                                 (SCREEN_WIDTH as uint * scale.factor()) as int,
                                 (SCREEN_HEIGHT as uint * scale.factor()) as int,
                                 INPUT_FOCUS).unwrap();
        let renderer = Renderer::from_window(window, RenderDriverIndex::Auto, ACCELERATED).unwrap();
        let texture = renderer.create_texture(BGR24,
                                              TextureAccess::Streaming,
                                              SCREEN_WIDTH as int,
                                              SCREEN_HEIGHT as int).unwrap();

        Gfx {
            renderer: box renderer,
            texture: box texture,
            scale: scale,
            status_line: StatusLine::new()
        }
    }

    pub fn tick(&mut self) {
        self.status_line.text.tick();
    }

    pub fn composite(&self, ppu_screen: &mut ([uint8_t, ..SCREEN_SIZE])) {
        self.status_line.render(ppu_screen);
        self.blit(&*ppu_screen);
        drop(self.renderer.clear());
        drop(self.renderer.copy(&*self.texture, None, Some(Rect {
            x: 0,
            y: 0,
            w: (SCREEN_WIDTH * self.scale.factor()) as int32_t,
            h: (SCREEN_HEIGHT * self.scale.factor()) as int32_t,
        })));
        self.renderer.present();
    }

    fn blit(&self, ppu_screen: &([uint8_t, ..SCREEN_SIZE])) {
        self.texture.update(None, ppu_screen, (SCREEN_WIDTH * 3) as int).unwrap()
    }
}
