//
// sprocketnes/gfx.rs
//
// Copyright (c) 2013 Mozilla Foundation
// Author: Patrick Walton
//

use sdl::sdl::{InitTimer, InitVideo};
use sdl::sdl;
use sdl::video::{DoubleBuf, HWSurface, Surface};
use sdl::video;

pub struct Gfx {
    screen: ~Surface
}

pub impl Gfx {
    static fn new() -> Gfx {
        sdl::init([ InitVideo, InitTimer ]);
        let screen = video::set_video_mode(320, 240, 24, [ HWSurface ], [ DoubleBuf ]).unwrap();
        Gfx { screen: screen }
    }
}

