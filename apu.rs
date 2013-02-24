//
// sprocketnes/apu.rs
//
// Author: Patrick Walton
//

use audio::OutputBuffer;
use mem::Mem;
use speex::Resampler;
use util::{Fd, Save};
use util;

use core::cast::{forget, transmute};
use core::libc::c_int;
use core::vec::each_mut;
use sdl::audio;

const CYCLES_PER_EVEN_TICK: u64 = 7438;
const CYCLES_PER_ODD_TICK: u64 = 7439;

const NES_SAMPLE_RATE: u32 = 1789920;   // Actual is 1789800, but this is divisible by 240.
const OUTPUT_SAMPLE_RATE: u32 = 44100;
const TICK_FREQUENCY: u32 = 240;
const NES_SAMPLES_PER_TICK: u32 = NES_SAMPLE_RATE / TICK_FREQUENCY;

const PULSE_WAVEFORMS: [u8 * 4] = [ 0b01000000, 0b01100000, 0b01111000, 0b10011111 ];

const LENGTH_COUNTERS: [u8 * 32] = [
    10, 254, 20,  2, 40,  4, 80,  6, 160,  8, 60, 10, 14, 12, 26, 14,
    12,  16, 24, 18, 48, 20, 96, 22, 192, 24, 72, 26, 16, 28, 32, 30,
];

//
// APUPULSE: [0x4000, 0x4008)
//

struct ApuPulseSweep(u8);

impl ApuPulseSweep {
    fn enabled(self) -> bool   { (*self >> 7) != 0         }
    fn period(self) -> u8      { ((*self >> 4) & 0x7) + 1  }
    fn negate(self) -> bool    { ((*self >> 3) & 0x1) != 0 }
    fn shift_count(self) -> u8 { *self & 0x7               }
}

struct ApuPulseEnvelope {
    disable_length: bool,
    enabled: bool,
    volume: u8,

    period: u8,
    counter: u8,
}

save_struct!(ApuPulseEnvelope { disable_length, enabled, volume, period, counter })

impl ApuPulseEnvelope {
    static fn new() -> ApuPulseEnvelope {
        ApuPulseEnvelope {
            disable_length: false,
            enabled: false,
            volume: 0,
            period: 0,
            counter: 0
        }
    }

    fn loops(self) -> bool { self.disable_length }
}

struct ApuPulse {
    duty: u8,
    envelope: ApuPulseEnvelope,
    sweep: ApuPulseSweep,
    timer: u16,

    length_id: u8,
    length_left: u8,
    sweep_cycle: u8,

    waveform_index: u8,
    wavelen_count: u64,
}

save_struct!(ApuPulse {
    duty, envelope, sweep, timer,
    length_id, length_left, sweep_cycle,
    waveform_index, wavelen_count
})

struct ApuStatus(u8);

impl ApuStatus {
    fn pulse_enabled(self, channel: u8) -> bool { ((*self >> channel) & 1) != 0 }
}

struct Regs {
    pulses: [ApuPulse * 2],
    status: ApuStatus,  // $4015: APUSTATUS
}

impl Save for Regs {
    fn save(&mut self, fd: &Fd) {
        self.pulses[0].save(fd);
        self.pulses[1].save(fd);
        self.status.save(fd);
    }
    fn load(&mut self, fd: &Fd) {
        self.pulses[0].load(fd);
        self.pulses[1].load(fd);
        self.status.load(fd);
    }
}

//
// Sample buffers
//

struct SampleBuffer {
    samples: [i16 * 178992],
}

//
// General operation
//

pub struct Apu {
    regs: Regs,

    sample_buffers: ~([SampleBuffer * 5]),
    sample_buffer_offset: uint,
    output_buffer: *mut OutputBuffer,
    resampler: Resampler,

    cy: u64,
    ticks: u64,
}

save_struct!(Apu { regs, cy, ticks })

impl Mem for Apu {
    fn loadb(&mut self, addr: u16) -> u8 {
        match addr {
            0x4015 => *self.regs.status,
            _ => 0
        }
    }
    fn storeb(&mut self, addr: u16, val: u8) {
        match addr {
            0x4000..0x4003 => self.update_pulse(addr, val, 0),
            0x4004..0x4007 => self.update_pulse(addr, val, 1),
            0x4015 => self.update_status(val),
            _ => {} // TODO
        }
    }
}

impl Apu {
    static pub fn new(output_buffer: *mut OutputBuffer) -> Apu {
        Apu {
            regs: Regs {
                pulses: [
                    ApuPulse {
                        duty: 0,
                        envelope: ApuPulseEnvelope::new(),
                        sweep: ApuPulseSweep(0),
                        timer: 0,

                        length_id: 0,
                        length_left: 0,
                        sweep_cycle: 0,

                        waveform_index: 0,
                        wavelen_count: 0,
                    }, ..2
                ],
                status: ApuStatus(0),
            },

            sample_buffers: ~[ SampleBuffer { samples: [ 0, ..178992 ] }, ..5 ],
            sample_buffer_offset: 0,
            output_buffer: output_buffer,
            resampler: Resampler::new(1, NES_SAMPLE_RATE, OUTPUT_SAMPLE_RATE, 0).unwrap(),

            cy: 0,
            ticks: 0,
        }
    }

    fn update_status(&mut self, val: u8) {
        self.regs.status = ApuStatus(val);

        for uint::range(0, 2) |i| {
            if !self.regs.status.pulse_enabled(i as u8) {
                self.regs.pulses[i].length_left = 0;
            }
        }
    }

    fn update_pulse(&mut self, addr: u16, val: u8, pulse_number: uint) {
        let pulse = &mut self.regs.pulses[pulse_number];
        match addr & 0x3 {
            0 => {
                pulse.duty = val >> 6;
                pulse.envelope.disable_length = ((val >> 5) & 1) != 0;
                pulse.envelope.enabled = ((val >> 4) & 1) == 0;
                if pulse.envelope.enabled {
                    pulse.envelope.volume = 15;
                    pulse.envelope.period = val & 0xf;
                    pulse.envelope.counter = 0;
                } else {
                    pulse.envelope.volume = val & 0xf;
                }
            }
            1 => {
                // TODO: Set reload flag.
                pulse.sweep = ApuPulseSweep(val);
                pulse.sweep_cycle = 0;
            }
            2 => pulse.timer = (pulse.timer & 0xff00) | (val as u16),
            3 => {
                pulse.length_id = val >> 3;

                // FIXME: Only set length_left if APUSTATUS has enabled this channel.
                pulse.length_left = LENGTH_COUNTERS[pulse.length_id];

                pulse.timer = (pulse.timer & 0x00ff) | ((val as u16 & 0x7) << 8);
            }
            _ => fail!(~"can't happen"),
        }
    }

    //
    // Playback
    //

    fn step(&mut self, run_to_cycle: u64) {
        loop {
            let mut next_tick_cycle = self.cy;
            if self.ticks % 2 == 0 {
                next_tick_cycle += CYCLES_PER_EVEN_TICK;
            } else {
                next_tick_cycle += CYCLES_PER_ODD_TICK;
            }

            if next_tick_cycle > run_to_cycle {
                break;
            }

            self.tick();

            self.cy = next_tick_cycle;
        }
    }

    fn tick(&mut self) {
        // 120 Hz operations: length counter and sweep.
        if self.ticks % 2 == 0 {
            // TODO: Remember that triangle wave has a different length disable bit.
            for uint::range(0, 2) |i| {
                let pulse = &mut self.regs.pulses[i];

                // Length counter.
                if pulse.length_left > 0 && !pulse.envelope.disable_length {
                    pulse.length_left -= 1;
                }

                // Sweep.
                pulse.sweep_cycle += 1;
                if pulse.sweep_cycle >= pulse.sweep.period() {
                    pulse.sweep_cycle = 0;

                    if pulse.sweep.enabled() {
                        let delta = pulse.timer >> pulse.sweep.shift_count();
                        if !pulse.sweep.negate() {
                            pulse.timer += delta;
                        } else {
                            pulse.timer -= delta;
                        }
                    }
                }
            }
        }

        // 240 Hz operations: envelope and linear counter.
        for uint::range(0, 2) |i| {
            let pulse = &mut self.regs.pulses[i];
            if pulse.envelope.enabled {
                pulse.envelope.counter += 1;
                if pulse.envelope.counter >= pulse.envelope.period {
                    pulse.envelope.counter = 0;
                    if pulse.envelope.volume == 0 {
                        if pulse.envelope.loops() {
                            pulse.envelope.volume = 15;
                        }
                    } else {
                        pulse.envelope.volume -= 1;
                        if pulse.envelope.volume == 0 && !pulse.envelope.loops() {
                            pulse.length_left = 0;
                        }
                    }
                }
            }
        }

        // Fill the sample buffers.
        self.play_pulse(0, 0);
        self.play_pulse(1, 1);
        self.sample_buffer_offset += NES_SAMPLES_PER_TICK as uint;

        // Now play the channels, flushing the sample buffers, if necessary.
        self.play_channels();

        // TODO: 60 Hz IRQ.

        self.ticks += 1;
    }

    fn play_pulse(&mut self, pulse_number: uint, channel: c_int) {
        let pulse = &mut self.regs.pulses[pulse_number];
        let timer = pulse.timer as uint;

        let mut sample_buffer = &mut self.sample_buffers[channel];
        let start_offset = self.sample_buffer_offset;
        let end_offset = start_offset + NES_SAMPLES_PER_TICK as uint;

        // Process sound.
        if timer > 0 && pulse.envelope.volume > 0 && pulse.length_left > 0 {
            let volume = (pulse.envelope.volume as i16 * 4) << 8;
            let wavelen = (pulse.timer as u64 + 1) * 2;
            let waveform: u8 = PULSE_WAVEFORMS[pulse.duty];

            // Fill the buffer.
            let mut buffer = vec::mut_slice(sample_buffer.samples, start_offset, end_offset);

            // TODO: Vectorize this for speed.
            let mut waveform_index = pulse.waveform_index;
            let mut wavelen_count = pulse.wavelen_count;

            for vec::each_mut(buffer) |dest| {
                wavelen_count += 1;
                if wavelen_count >= wavelen {
                    wavelen_count = 0;
                    waveform_index = (waveform_index + 1) % 8;
                }

                *dest = if ((waveform >> (7 - waveform_index)) & 1) != 0 { volume } else { 0 };
            }

            pulse.waveform_index = waveform_index;
            pulse.wavelen_count = wavelen_count;
        } else {
            for uint::range(start_offset, end_offset) |i| {
                sample_buffer.samples[i] = 0;
            }
        }
    }

    // Resamples and flushes channel buffers to the audio output device if necessary.
    fn play_channels(&mut self) {
        let sample_buffer_length = self.sample_buffers[0].samples.len();
        if self.sample_buffer_offset < sample_buffer_length {
            return;
        }
        self.sample_buffer_offset = 0;

        // First, mix all sample buffers into the first one.
        //
        // FIXME: This should not be a linear mix, for accuracy.
        for uint::range(0, self.sample_buffers[0].samples.len()) |i| {
            let mut val = 0;
            for uint::range(0, 5) |j| {
                val += self.sample_buffers[j].samples[i] as i32;
            }

            if val > 32767 {
                val = 32767;
            } else if val < -32768 {
                val = -32768;
            }

            self.sample_buffers[0].samples[i] = val as i16;
        }

        // Wait for the audio callback to catch up if necessary.
        // FIXME: This is a racy spinlock; use condvars instead.
        loop {
            let played = do audio::with_lock {
                unsafe {
                    (*self.output_buffer).play_offset == (*self.output_buffer).samples.len()
                }
            };
            if played {
                break;
            }
        }

        // Resample and output the audio.
        do audio::with_lock {
            unsafe {
                let _ = self.resampler.process(0,
                                               self.sample_buffers[0].samples,
                                               (*self.output_buffer).samples);
                (*self.output_buffer).play_offset = 0;
            }
        }
    }
}

