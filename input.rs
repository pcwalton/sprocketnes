//
// sprocketnes/input.rs
//
// Author: Patrick Walton
//

use mem::Mem;

use libc::{uint8_t, uint16_t};
use sdl2::Sdl;
use sdl2::event::Event;
use sdl2::event::Event::*;
use sdl2::keyboard::Keycode;

use std::ops::Deref;

//
// The "strobe state": the order in which the NES reads the buttons.
//

const STROBE_STATE_A: uint8_t        = 0;
const STROBE_STATE_B: uint8_t        = 1;
const STROBE_STATE_SELECT: uint8_t   = 2;
const STROBE_STATE_START: uint8_t    = 3;
const STROBE_STATE_UP: uint8_t       = 4;
const STROBE_STATE_DOWN: uint8_t     = 5;
const STROBE_STATE_LEFT: uint8_t     = 6;
const STROBE_STATE_RIGHT: uint8_t    = 7;

struct StrobeState{ val: uint8_t }

impl Deref for StrobeState {
    type Target = uint8_t;

    fn deref(&self) -> &uint8_t {
        &self.val
    }
}

impl StrobeState {
    // Given a GamePadState structure, returns the state of the given button.
    fn get(&self, state: &GamePadState) -> bool {
        match **self {
            STROBE_STATE_A      => state.a,
            STROBE_STATE_B      => state.b,
            STROBE_STATE_SELECT => state.select,
            STROBE_STATE_START  => state.start,
            STROBE_STATE_UP     => state.up,
            STROBE_STATE_DOWN   => state.down,
            STROBE_STATE_LEFT   => state.left,
            STROBE_STATE_RIGHT  => state.right,
            _                   => panic!("shouldn't happen")
        }
    }

    fn next(&mut self) {
        *self = StrobeState{val: (**self + 1) & 7};
    }

    fn reset(&mut self) {
        *self = StrobeState{val: STROBE_STATE_A};
    }
}

//
// The standard NES game pad state
//

pub struct GamePadState {
    pub left: bool,
    pub down: bool,
    pub up: bool,
    pub right: bool,
    pub a: bool,
    pub b: bool,
    pub select: bool,
    pub start: bool,

    strobe_state: StrobeState,
}

pub struct Input {
    pub gamepad_0: GamePadState,
    sdl: Sdl,   // FIXME: Use a `&'a mut EventPump` instead
}

pub enum InputResult {
    Continue,   // Keep playing.
    Quit,       // Quit the emulator.
    SaveState,  // Save a state.
    LoadState,  // Load a state.
}

impl Input {
    pub fn new(sdl: Sdl) -> Input {
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

                strobe_state: StrobeState{val: STROBE_STATE_A}
            },
            sdl: sdl,
        }
    }

    fn handle_gamepad_event(&mut self, key: Keycode, down: bool) {
        match key {
            Keycode::Left   => self.gamepad_0.left   = down,
            Keycode::Down   => self.gamepad_0.down   = down,
            Keycode::Up     => self.gamepad_0.up     = down,
            Keycode::Right  => self.gamepad_0.right  = down,
            Keycode::Z      => self.gamepad_0.a      = down,
            Keycode::X      => self.gamepad_0.b      = down,
            Keycode::RShift => self.gamepad_0.select = down,
            Keycode::Return => self.gamepad_0.start  = down,
            _               => {}
        }
    }

    pub fn check_input(&mut self) -> InputResult {
        while let Some(ev) = self.sdl.event_pump().poll_event() {
            match ev {
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => return InputResult::Quit,
                Event::KeyDown { keycode: Some(Keycode::S), .. } => return InputResult::SaveState,
                Event::KeyDown { keycode: Some(Keycode::L), .. } => return InputResult::LoadState,
                Event::KeyDown { keycode: Some(key), .. } => {
                    self.handle_gamepad_event(key, true)
                }
                Event::KeyUp { keycode: Some(key), .. } => self.handle_gamepad_event(key, false),
                Event::Quit { .. } => return InputResult::Quit,
                _ => {}
            }
        }

        return InputResult::Continue;
    }
}

impl Mem for Input {
    fn loadb(&mut self, addr: uint16_t) -> uint8_t {
        if addr == 0x4016 {
            let result = self.gamepad_0.strobe_state.get(&self.gamepad_0) as uint8_t;
            self.gamepad_0.strobe_state.next();
            result
        } else {
            0
        }
    }

    fn storeb(&mut self, addr: uint16_t, _: uint8_t) {
        if addr == 0x4016 {
            // FIXME: This is not really accurate; you're supposed to not reset until you see
            // 1 strobed than 0. But I doubt this will break anything.
            self.gamepad_0.strobe_state.reset();
        }
    }
}
