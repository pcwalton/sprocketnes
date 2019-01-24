//! SDL audio interface. Used by the APU to actually play audio.

//
// Author: Patrick Walton
//

// TODO: This module is very unsafe. Adding a reader-writer audio lock to SDL would help make it
// safe.

use sdl2::audio::{AudioCallback, AudioDevice, AudioDeviceLockGuard, AudioSpecDesired};
use sdl2::Sdl;
use std::cmp;
use std::mem;
use std::slice::from_raw_parts_mut;
use std::sync::{Condvar, Mutex};

//
// The audio callback
//

const SAMPLE_COUNT: usize = 4410 * 2;

static mut G_AUDIO_DEVICE: Option<*mut AudioDevice<NesAudioCallback>> = None;

static mut G_OUTPUT_BUFFER: Option<*mut OutputBuffer> = None;

lazy_static! {
    pub static ref AUDIO_MUTEX: Mutex<()> = Mutex::new(());
    pub static ref AUDIO_CONDVAR: Condvar = Condvar::new();
}

pub struct OutputBuffer {
    pub samples: [u8; SAMPLE_COUNT],
    pub play_offset: usize,
}

pub struct NesAudioCallback;

impl AudioCallback for NesAudioCallback {
    type Channel = i16;

    fn callback(&mut self, buf: &mut [Self::Channel]) {
        unsafe {
            let samples: &mut [u8] =
                from_raw_parts_mut(&mut buf[0] as *mut i16 as *mut u8, buf.len() * 2);
            let output_buffer: &mut OutputBuffer = mem::transmute(G_OUTPUT_BUFFER.unwrap());
            let play_offset = output_buffer.play_offset;
            let output_buffer_len = output_buffer.samples.len();

            for i in 0..samples.len() {
                if i + play_offset >= output_buffer_len {
                    break;
                }
                samples[i] = output_buffer.samples[i + play_offset];
            }

            let _ = AUDIO_MUTEX.lock();
            output_buffer.play_offset = cmp::min(play_offset + samples.len(), output_buffer_len);
            AUDIO_CONDVAR.notify_one();
        }
    }
}

/// Audio initialization. If successful, returns a pointer to an allocated `OutputBuffer` that can
/// be filled with raw audio data.
pub fn open(sdl: &Sdl) -> Option<*mut OutputBuffer> {
    let output_buffer = Box::new(OutputBuffer {
        samples: [0; SAMPLE_COUNT],
        play_offset: 0,
    });
    let output_buffer_ptr: *mut OutputBuffer = unsafe { mem::transmute(&*output_buffer) };

    unsafe {
        G_OUTPUT_BUFFER = Some(output_buffer_ptr);
        mem::forget(output_buffer);
    }

    let spec = AudioSpecDesired {
        freq: Some(44100),
        channels: Some(1),
        samples: Some(4410),
    };

    let audio_subsystem = sdl.audio().unwrap();
    unsafe {
        match audio_subsystem.open_playback(None, &spec, |_| NesAudioCallback) {
            Ok(device) => {
                device.resume();
                G_AUDIO_DEVICE = Some(mem::transmute(Box::new(device)));
                return Some(output_buffer_ptr);
            }
            Err(e) => {
                println!("Error initializing AudioDevice: {}", e);
                return None;
            }
        }
    }
}

//
// Audio tear-down
//

pub fn close() {
    unsafe {
        match G_AUDIO_DEVICE {
            None => {}
            Some(ptr) => {
                let _: Box<AudioDevice<NesAudioCallback>> = mem::transmute(ptr);
                G_AUDIO_DEVICE = None;
            }
        }
    }
}

pub fn lock<'a>() -> Option<AudioDeviceLockGuard<'a, NesAudioCallback>> {
    unsafe { G_AUDIO_DEVICE.map(|dev| (*dev).lock()) }
}
