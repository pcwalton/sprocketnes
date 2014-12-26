//
// sprocketnes/audio.rs
//
// Author: Patrick Walton
//

// TODO: This module is very unsafe. Adding a reader-writer audio lock to SDL would help make it
// safe.

use libc::{c_int, c_void, uint8_t};
//use sdl2::audio::ll::{ SDL_AudioSpec, AUDIO_S16LSB };
//use sdl2::audio::AudioDevice;
use std::cmp;
use std::mem;
//use std::ptr;
use std::raw::Slice;
use std::sync::{MUTEX_INIT, CONDVAR_INIT, StaticMutex, StaticCondvar};

//
// The audio callback
//

const SAMPLE_COUNT: uint = 4410 * 2;

//static mut g_audio_device: Option<AudioDevice> = None;

static mut g_output_buffer: Option<*mut OutputBuffer> = None;

pub static mut g_mutex: StaticMutex = MUTEX_INIT;
pub static mut g_condvar: StaticCondvar = CONDVAR_INIT;

#[allow(missing_copy_implementations)]
pub struct OutputBuffer {
    pub samples: [uint8_t, .. SAMPLE_COUNT],
    pub play_offset: uint,
}

#[allow(dead_code)]
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
/*
    let spec = SDL_AudioSpec {
        freq: 44100,
        format: AUDIO_S16LSB,
        channels: 1,
        silence: 0,
        samples: 4410,
        padding: 0,
        size: 0,
        userdata: ptr::null(),
        callback: Some(nes_audio_callback),
    };

    unsafe {
        match AudioDevice::open(None, 0, mem::transmute(&spec)) {
            Ok(x) => {
                let (device, _) = x;
                device.resume();
                g_audio_device = Some(device);
                return Some(output_buffer_ptr)
            },
            Err(e) => {
                println!("Error initializing AudioDevice: {}", e);*/
    println!("FIXME: audio init");
                return None
            /*}
        }
    }*/
}

//
// Audio tear-down
//

pub fn close() {
        /*
    unsafe {
        match g_audio_device {
            None => {}
            Some(audio_device) => {
                audio_device.close();
                g_audio_device = None
            }
}
    }
*/

}

pub struct AudioLock;

impl Drop for AudioLock {
    fn drop(&mut self) {
        /*
        unsafe {
            match g_audio_device {
                None => {}
                Some(audio_device) =>
                    audio_device.unlock(),
            }
        }
        */
    }
}

impl AudioLock {
    pub fn lock() -> AudioLock {
        /*
        unsafe {
            match g_audio_device {
                None => {}
                Some(audio_device) => audio_device.lock(),
            }
        }
        */
        AudioLock
    }
}
