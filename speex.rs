//
// sprocketnes/speex.rs
//
// Author: Patrick Walton
//

use libc::{c_int, c_void, int16_t, uint8_t, uint32_t};
use std::cast::transmute;
use std::ptr::null;

type SpeexResamplerState = c_void;

#[link_args="-lspeexdsp"]
extern {
    fn speex_resampler_init(nb_channels: uint32_t,
                            in_rate: uint32_t,
                            out_rate: uint32_t,
                            quality: c_int,
                            err: *mut c_int)
                            -> *SpeexResamplerState;
    fn speex_resampler_destroy(st: *SpeexResamplerState);
    fn speex_resampler_process_int(st: *SpeexResamplerState,
                                   channel_index: uint32_t,
                                   input: *int16_t,
                                   in_len: *mut uint32_t,
                                   out: *int16_t,
                                   out_len: *mut uint32_t)
                                   -> c_int;
}

pub struct Resampler {
    speex_resampler: *SpeexResamplerState,
}

impl Resampler {
    pub fn new(channels: uint32_t, in_rate: uint32_t, out_rate: uint32_t, quality: c_int)
               -> Result<Resampler,c_int> {
        unsafe {
            let mut err = 0;
            let speex_resampler = speex_resampler_init(channels,
                                                       in_rate,
                                                       out_rate,
                                                       quality,
                                                       &mut err);
            if speex_resampler == null() {
                Err(err)
            } else {
                Ok(Resampler {
                    speex_resampler: speex_resampler,
                })
            }
        }
    }

    pub fn process(&self, channel_index: uint32_t, input: &[int16_t], out: &mut [uint8_t])
                   -> (uint32_t, uint32_t) {
        unsafe {
            assert!(input.len() <= 0xffffffff);
            assert!(out.len() / 2 <= 0xffffffff);
            let (in_len, out_len) = (input.len() as uint32_t, out.len() as uint32_t / 2);
            let mut in_len = in_len;
            let mut out_len = out_len;
            let err = speex_resampler_process_int(self.speex_resampler,
                                                  channel_index,
                                                  &input[0],
                                                  &mut in_len,
                                                  transmute(&out[0]),
                                                  &mut out_len);
            assert!(err == 0);
            (in_len, out_len)
        }
    }
}

impl Drop for Resampler {
    fn drop(&mut self) {
        unsafe {
            speex_resampler_destroy(self.speex_resampler)
        }
    }
}

