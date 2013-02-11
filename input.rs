//
// sprocketnes/input.rs
//
// Author: Patrick Walton
//

use mem::Mem;

use sdl::event::{KeyDownEvent, KeyUpEvent, KeyboardEvent, NoEvent, QuitEvent};
use sdl::event;
use sdl::keyboard::{SDLKDown, SDLKEscape, SDLKLeft, SDLKRShift, SDLKReturn, SDLKRight, SDLKUp};
use sdl::keyboard::{SDLKx, SDLKz};

//
// The "strobe state": the order in which the NES reads the buttons.
//

const STROBE_STATE_A: u8        = 0;
const STROBE_STATE_B: u8        = 1;
const STROBE_STATE_SELECT: u8   = 2;
const STROBE_STATE_START: u8    = 3;
const STROBE_STATE_UP: u8       = 4;
const STROBE_STATE_DOWN: u8     = 5;
const STROBE_STATE_LEFT: u8     = 6;
const STROBE_STATE_RIGHT: u8    = 7;

struct StrobeState(u8);

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
            _                   => die!(~"shouldn't happen")
        }
    }

    fn next(&mut self) {
        *self = StrobeState((**self + 1) & 7);
    }

    fn reset(&mut self) {
        *self = StrobeState(STROBE_STATE_A);
    }
}

//
// The standard NES game pad state
//

pub struct GamePadState {
    left: bool,
    down: bool,
    up: bool,
    right: bool,
    a: bool,
    b: bool,
    select: bool,
    start: bool,

    strobe_state: StrobeState,
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

                strobe_state: StrobeState(STROBE_STATE_A)
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
                QuitEvent => return Quit,
                _ => {}
            }
        }
        return Continue;
    }
}

impl Mem for Input {
    fn loadb(&mut self, addr: u16) -> u8 {
        if addr == 0x4016 {
            let result = self.gamepad_0.strobe_state.get(&self.gamepad_0) as u8;
            self.gamepad_0.strobe_state.next();
            result
        } else {
            0
        }
    }

    fn storeb(&mut self, addr: u16, _: u8) {
        if addr == 0x4016 {
            // FIXME: This is not really accurate; you're supposed to not reset until you see
            // 1 strobed than 0. But I doubt this will break anything.
            self.gamepad_0.strobe_state.reset();
        }
    }
}

