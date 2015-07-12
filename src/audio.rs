/// SDL audio interface. Used by the APU to actually play audio.

//
// Author: Patrick Walton
//

// TODO: This module is very unsafe. Adding a reader-writer audio lock to SDL would help make it
// safe.

use sdl2::audio::{AudioDevice, AudioCallback, AudioSpecDesired, AudioDeviceLockGuard};
use std::cmp;
use std::mem;
use std::slice::from_raw_parts_mut;
use std::sync::{StaticMutex, StaticCondvar, MUTEX_INIT, CONDVAR_INIT};

//
// The audio callback
//

const SAMPLE_COUNT: usize = 4410 * 2;

static mut g_audio_device: Option<*mut AudioDevice<NesAudioCallback>> = None;

static mut g_output_buffer: Option<*mut OutputBuffer> = None;

pub static mut g_mutex: StaticMutex = MUTEX_INIT;

pub static mut g_condvar: StaticCondvar = CONDVAR_INIT;

pub struct OutputBuffer {
    pub samples: [u8; SAMPLE_COUNT],
    pub play_offset: usize,
}

pub struct NesAudioCallback;

impl AudioCallback for NesAudioCallback {
    type Channel = i16;

    fn callback(&mut self, buf: &mut [Self::Channel]) {
        unsafe {
            let samples: &mut [u8] = from_raw_parts_mut(&mut buf[0] as *mut i16 as *mut u8, buf.len() * 2);
            let output_buffer: &mut OutputBuffer = mem::transmute(g_output_buffer.unwrap());
            let play_offset = output_buffer.play_offset;
            let output_buffer_len = output_buffer.samples.len();

            for i in 0..samples.len() {
                if i + play_offset >= output_buffer_len {
                    break;
                }
                samples[i] = output_buffer.samples[i + play_offset];
            }

            let _ = g_mutex.lock();
            output_buffer.play_offset = cmp::min(play_offset + samples.len(), output_buffer_len);
            g_condvar.notify_one();
        }
    }
}

/// Audio initialization. If successful, returns a pointer to an allocated `OutputBuffer` that can
/// be filled with raw audio data.
pub fn open() -> Option<*mut OutputBuffer> {
    let output_buffer = Box::new(OutputBuffer {
        samples: [ 0; SAMPLE_COUNT ],
        play_offset: 0,
    });
    let output_buffer_ptr: *mut OutputBuffer = unsafe {
        mem::transmute(&*output_buffer)
    };

    unsafe {
        g_output_buffer = Some(output_buffer_ptr);
        mem::forget(output_buffer);
    }

    let spec = AudioSpecDesired {
        freq: Some(44100),
        channels: Some(1),
        samples: Some(4410),
    };

    unsafe {
        match AudioDevice::open_playback(None, spec, |_| NesAudioCallback) {
            Ok(device) => {
                device.resume();
                g_audio_device = Some(mem::transmute(Box::new(device)));
                return Some(output_buffer_ptr)
            },
            Err(e) => {
                println!("Error initializing AudioDevice: {}", e);
                return None
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
            Some(ptr) => {
                let _: Box<AudioDevice<NesAudioCallback>> = mem::transmute(ptr);
                g_audio_device = None;
            }
        }
    }
}

pub fn lock<'a>() -> Option<AudioDeviceLockGuard<'a, NesAudioCallback>> {
    unsafe {
        g_audio_device.map(|dev| (*dev).lock())
    }
}
