//
// sprocketnes/input.rs
//
// Copyright (c) 2012 Mozilla Foundation
// Author: Patrick Walton
//

use sdl::event::{KeyDownEvent, KeyUpEvent, KeyboardEvent, NoEvent};
use sdl::event;
use sdl::keyboard::{SDLKDown, SDLKEscape, SDLKLeft, SDLKRShift, SDLKReturn, SDLKRight, SDLKUp};
use sdl::keyboard::{SDLKx, SDLKz};

pub struct GamePadState {
    left: bool,
    down: bool,
    up: bool,
    right: bool,
    a: bool,
    b: bool,
    select: bool,
    start: bool
}

pub struct Input {
    gamepad_0: GamePadState
}

pub enum InputResult {
    Continue,   // Keep playing.
    Quit,       // Quit the emulator.
}

impl Input {
    static fn new() -> Input {
        Input {
            gamepad_0: GamePadState {
                left: false,
                down: false,
                up: false,
                right: false,
                a: false,
                b: false,
                select: false,
                start: false,
            }
        }
    }

    fn handle_gamepad_event(&mut self, key_event: &KeyboardEvent, down: bool) {
        match key_event.keycode {
            SDLKLeft   => self.gamepad_0.left   = down,
            SDLKDown   => self.gamepad_0.down   = down,
            SDLKUp     => self.gamepad_0.up     = down,
            SDLKRight  => self.gamepad_0.right  = down,
            SDLKz      => self.gamepad_0.a      = down,
            SDLKx      => self.gamepad_0.b      = down,
            SDLKRShift => self.gamepad_0.select = down,
            SDLKReturn => self.gamepad_0.start  = down,
            _          => {}
        }
    }

    fn check_input(&mut self) -> InputResult {
        loop {
            match event::poll_event() {
                NoEvent => break,
                KeyDownEvent(ref key_event) => {
                    self.handle_gamepad_event(key_event, true);

                    if key_event.keycode == SDLKEscape {
                        return Quit;
                    }
                }
                KeyUpEvent(ref key_event) => self.handle_gamepad_event(key_event, false),
                _ => {}
            }
        }
        return Continue;
    }
}

