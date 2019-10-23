//! SDL audio interface. Used by the APU to actually play audio.

//
// Author: Patrick Walton
//

// TODO: This module is very unsafe. Adding a reader-writer audio lock to SDL would help make it
// safe.

use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired};
use sdl2::Sdl;
use std::cmp;
use std::slice::from_raw_parts_mut;
use std::sync::{Condvar, Mutex};

//
// The audio callback
//

const SAMPLE_COUNT: usize = 4410 * 2;

lazy_static! {
    pub static ref AUDIO_MUTEX: Mutex<()> = Mutex::new(());
    pub static ref AUDIO_CONDVAR: Condvar = Condvar::new();
}

pub struct NesAudioCallback {
    pub samples: [u8; SAMPLE_COUNT],
    pub play_offset: usize,
}

impl AudioCallback for NesAudioCallback {
    type Channel = i16;

    fn callback(&mut self, buf: &mut [Self::Channel]) {
        unsafe {
            let samples: &mut [u8] =
                from_raw_parts_mut(&mut buf[0] as *mut i16 as *mut u8, buf.len() * 2);
            let play_offset = self.play_offset;
            let output_buffer_len = self.samples.len();

            for i in 0..samples.len() {
                if i + play_offset >= output_buffer_len {
                    break;
                }
                samples[i] = self.samples[i + play_offset];
            }

            let _ = AUDIO_MUTEX.lock();
            self.play_offset = cmp::min(play_offset + samples.len(), output_buffer_len);
            AUDIO_CONDVAR.notify_one();
        }
    }
}

/// Audio initialization. If successful, returns an SDL AudioDevice that can be used (by locking)
/// to get an output buffer reference to be filled with raw audio data.
pub fn open(sdl: &Sdl) -> Option<AudioDevice<NesAudioCallback>> {
    let spec = AudioSpecDesired {
        freq: Some(44100),
        channels: Some(1),
        samples: Some(4410),
    };

    let audio_subsystem = sdl.audio().unwrap();
    match audio_subsystem.open_playback(None, &spec, |_| NesAudioCallback {
        samples: [0; SAMPLE_COUNT],
        play_offset: 0,
    }) {
        Ok(device) => {
            device.resume();
            return Some(device);
        }
        Err(e) => {
            println!("Error initializing AudioDevice: {}", e);
            return None;
        }
    }
}
