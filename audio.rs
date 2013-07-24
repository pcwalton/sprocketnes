//
// sprocketnes/audio.rs
//
// Author: Patrick Walton
//

// TODO: This module is very unsafe. Adding a reader-writer audio lock to SDL would help make it
// safe.

use sdl::audio::{DesiredAudioSpec, Mono, S16LsbAudioFormat};
use sdl::audio;
use std::cast::{forget, transmute};
use std::uint;

//
// The audio callback
//

static SAMPLE_COUNT: uint = 4410 * 2;

pub struct OutputBuffer {
    samples: [u8, ..SAMPLE_COUNT],
    play_offset: uint,
}

fn audio_callback(samples: &mut [u8], output_buffer: &mut OutputBuffer) {
    let play_offset = output_buffer.play_offset;
    let output_buffer_len = output_buffer.samples.len();

    for uint::range(0, samples.len()) |i| {
        if i + play_offset >= output_buffer_len {
            break;
        }
        samples[i] = output_buffer.samples[i + play_offset];
    }

    output_buffer.play_offset = uint::min(play_offset + samples.len(), output_buffer_len);
}

//
// Audio initialization
//

pub fn open() -> *mut OutputBuffer {
    let output_buffer = ~OutputBuffer { samples: [ 0, ..8820 ], play_offset: 0 };
    let output_buffer_ptr: *mut OutputBuffer = unsafe { transmute(&*output_buffer) };

    let spec = DesiredAudioSpec {
        freq: 44100,
        format: S16LsbAudioFormat,
        channels: Mono,
        samples: 4410,
        callback: |samples| {
            unsafe {
                audio_callback(samples, transmute(output_buffer_ptr))
            }
        },
    };
    assert!(audio::open(spec).is_ok());
    audio::pause(false);

    unsafe {
        forget(output_buffer);
    }

    output_buffer_ptr
}

//
// Audio tear-down
//

pub fn close() {
    audio::close();
}

