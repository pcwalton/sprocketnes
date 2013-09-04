//
// sprocketnes/speex.rs
//
// Author: Patrick Walton
//

use std::cast::transmute;
use std::libc::{c_int, c_void};
use std::ptr::null;

type SpeexResamplerState = c_void;

#[link_args="-lspeexdsp"]
extern {
    fn speex_resampler_init(nb_channels: u32,
                            in_rate: u32,
                            out_rate: u32,
                            quality: c_int,
                            err: *mut c_int)
                            -> *SpeexResamplerState;
    fn speex_resampler_destroy(st: *SpeexResamplerState);
    fn speex_resampler_process_int(st: *SpeexResamplerState,
                                   channel_index: u32,
                                   input: *i16,
                                   in_len: *mut u32,
                                   out: *i16,
                                   out_len: *mut u32)
                                   -> c_int;
    fn speex_resampler_reset_mem(st: *SpeexResamplerState);
    fn speex_resampler_get_rate(st: *SpeexResamplerState,
                                in_rate: *mut u32,
                                out_rate: *mut u32);
}

pub struct Resampler {
    priv speex_resampler: *SpeexResamplerState,
}

impl Resampler {
    #[fixed_stack_segment]
    pub fn new(channels: u32, in_rate: u32, out_rate: u32, quality: c_int)
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

    #[fixed_stack_segment]
    pub fn process(&self, channel_index: u32, input: &[i16], out: &mut [u8]) -> (u32, u32) {
        unsafe {
            assert!(input.len() <= 0xffffffff);
            assert!(out.len() / 2 <= 0xffffffff);
            let (in_len, out_len) = (input.len() as u32, out.len() as u32 / 2);
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
    #[fixed_stack_segment]
    fn drop(&self) {
        unsafe {
            speex_resampler_destroy(self.speex_resampler)
        }
    }
}

