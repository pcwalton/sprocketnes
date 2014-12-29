//
// sprocketnes/audio.rs
//
// Author: Patrick Walton
//

// TODO: This module is very unsafe. Adding a reader-writer audio lock to SDL would help make it
// safe.

use libc::{c_int, c_void, uint8_t};
use sdl2::audio::ll;
use std::cmp;
use std::mem;
use std::ptr;
use std::raw::Slice;
use std::sync::{MUTEX_INIT, CONDVAR_INIT, StaticMutex, StaticCondvar};

//
// The audio callback
//

const SAMPLE_COUNT: uint = 4410 * 2;

type AudioDeviceID = u32;

static mut g_audio_device: Option<AudioDeviceID> = None;

static mut g_output_buffer: Option<*mut OutputBuffer> = None;

pub static mut g_mutex: StaticMutex = MUTEX_INIT;
pub static mut g_condvar: StaticCondvar = CONDVAR_INIT;

#[allow(missing_copy_implementations)]
pub struct OutputBuffer {
    pub samples: [uint8_t, .. SAMPLE_COUNT],
    pub play_offset: uint,
}

extern "C" fn nes_audio_callback(_: *const c_void,
                                 stream: *const uint8_t,
                                 len: c_int) {
    unsafe {
        let samples: &mut [uint8_t] = mem::transmute(Slice {
            data: stream,
            len: len as uint,
        });

        let output_buffer: &mut OutputBuffer = mem::transmute(g_output_buffer.unwrap());
        let play_offset = output_buffer.play_offset;
        let output_buffer_len = output_buffer.samples.len();

        for i in range(0, samples.len()) {
            if i + play_offset >= output_buffer_len {
                break;
            }
            samples[i] = output_buffer.samples[i + play_offset];
        }

        output_buffer.play_offset = cmp::min(play_offset + samples.len(), output_buffer_len);
        g_condvar.notify_all();
    }
}

//
// Audio initialization
//

pub fn open() -> Option<*mut OutputBuffer> {
    let output_buffer = box OutputBuffer {
        samples: [ 0, ..8820 ],
        play_offset: 0,
    };
    let output_buffer_ptr: *mut OutputBuffer = unsafe {
        mem::transmute(&*output_buffer)
    };

    unsafe {
        g_output_buffer = Some(output_buffer_ptr);
        mem::forget(output_buffer);
    }

    let spec = ll::SDL_AudioSpec {
        freq: 44100,
        format: ll::AUDIO_S16LSB,
        channels: 1,
        silence: 0,
        samples: 4410,
        padding: 0,
        size: 0,
        userdata: ptr::null(),
        callback: Some(nes_audio_callback as extern "C" fn(*const c_void, *const u8, i32)),
    };

    unsafe {
        use std::mem::uninitialized;
        use sdl2;

        let mut obtained = uninitialized::<ll::SDL_AudioSpec>();

        match ll::SDL_OpenAudioDevice(ptr::null(), 0, &spec, &mut obtained, 0) {
            0 => {
                println!("Error initializing AudioDevice: {}", sdl2::get_error());
                None
            },
            device_id => {
                // start playing
                ll::SDL_PauseAudioDevice(device_id, 0);
                g_audio_device = Some(device_id);
                Some(output_buffer_ptr)
            }
        }
    }
}

//
// Audio tear-down
//

pub fn close() {
    unsafe {
        match g_audio_device {
            None => {}
            Some(audio_device) => {
                ll::SDL_CloseAudioDevice(audio_device);
                g_audio_device = None
            }
        }
    }

}

pub struct AudioLock;

impl Drop for AudioLock {
    fn drop(&mut self) {
        unsafe {
            match g_audio_device {
                None => {},
                Some(audio_device) => ll::SDL_UnlockAudioDevice(audio_device)
            }
        }
    }
}

impl AudioLock {
    pub fn lock() -> AudioLock {
        unsafe {
            match g_audio_device {
                None => {},
                Some(audio_device) => ll::SDL_LockAudioDevice(audio_device)
            }
        }
        AudioLock
    }
}
