//! A wrapper for the resampling module in Speex.

//
// Author: Patrick Walton
//

use libc::{c_int, c_void, int16_t, uint32_t};
use std::mem::transmute;
use std::ptr::null;

type SpeexResamplerState = c_void;

#[link(name = "speexdsp")]
extern "C" {
    fn speex_resampler_init(
        nb_channels: uint32_t,
        in_rate: uint32_t,
        out_rate: uint32_t,
        quality: c_int,
        err: *mut c_int,
    ) -> *const SpeexResamplerState;
    fn speex_resampler_destroy(st: *const SpeexResamplerState);
    fn speex_resampler_process_int(
        st: *const SpeexResamplerState,
        channel_index: uint32_t,
        input: *const int16_t,
        in_len: *mut uint32_t,
        out: *const int16_t,
        out_len: *mut uint32_t,
    ) -> c_int;
}

pub struct Resampler {
    speex_resampler: *const SpeexResamplerState,
}

impl Resampler {
    /// Creates a new resampler that will resample the input stream from `in_rate` to `out_rate`.
    /// The resampling quality can be an integer in range `0..10` (inclusive), where 10 is the
    /// highest quality.
    pub fn new(
        channels: u32,
        in_rate: u32,
        out_rate: u32,
        quality: c_int,
    ) -> Result<Resampler, c_int> {
        unsafe {
            let mut err = 0;
            let speex_resampler =
                speex_resampler_init(channels, in_rate, out_rate, quality, &mut err);
            if speex_resampler == null() {
                Err(err)
            } else {
                Ok(Resampler {
                    speex_resampler: speex_resampler,
                })
            }
        }
    }

    /// Resamples `input` on channel `channel_index` and writes the result to `out`.
    ///
    /// Returns a tuple of the number of input samples processed and output samples written.
    pub fn process(&self, channel_index: u32, input: &[i16], out: &mut [u8]) -> (u32, u32) {
        unsafe {
            assert!(input.len() <= 0xffffffff);
            assert!(out.len() / 2 <= 0xffffffff);
            let (in_len, out_len) = (input.len() as u32, out.len() as u32 / 2);
            let mut in_len = in_len;
            let mut out_len = out_len;
            let err = speex_resampler_process_int(
                self.speex_resampler,
                channel_index,
                &input[0],
                &mut in_len,
                transmute(&out[0]),
                &mut out_len,
            );
            assert!(err == 0);
            (in_len, out_len)
        }
    }
}

impl Drop for Resampler {
    fn drop(&mut self) {
        unsafe { speex_resampler_destroy(self.speex_resampler) }
    }
}
