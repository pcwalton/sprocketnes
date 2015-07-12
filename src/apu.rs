//! The Audio Processing Unit (APU).

//
// Author: Patrick Walton
//

use audio::{self, OutputBuffer};
use mem::Mem;
use speex::Resampler;
use util::{Save, Xorshift};

use std::fs::File;
use std::ops::{Deref, DerefMut};

const CYCLES_PER_EVEN_TICK: u64 = 7438;
const CYCLES_PER_ODD_TICK: u64 = 7439;

const NES_SAMPLE_RATE: u32 = 1789920;   // Actual is 1789800, but this is divisible by 240.
const OUTPUT_SAMPLE_RATE: u32 = 44100;
const TICK_FREQUENCY: u32 = 240;
const NES_SAMPLES_PER_TICK: u32 = NES_SAMPLE_RATE / TICK_FREQUENCY;

const PULSE_WAVEFORMS: [u8; 4] = [ 0b01000000, 0b01100000, 0b01111000, 0b10011111 ];

const LENGTH_COUNTERS: [u8; 32] = [
    10, 254, 20,  2, 40,  4, 80,  6, 160,  8, 60, 10, 14, 12, 26, 14,
    12,  16, 24, 18, 48, 20, 96, 22, 192, 24, 72, 26, 16, 28, 32, 30,
];

const TRIANGLE_WAVEFORM: [u8; 32] = [
    15, 14, 13, 12, 11, 10,  9,  8,  7,  6,  5,  4,  3,  2,  1,  0,
     0,  1,  2,  3,  4,  5,  6,  7,  8,  9, 10, 11, 12, 13, 14, 15,
];

// TODO: PAL
const NOISE_PERIODS: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068
];

//
// Channel lengths
//

// There are two modes in which the disable bit can be set: bit 5 (pulses) or bit 7 (triangle).
trait DisableBit { fn bit_number(self) -> u8; }
struct DisableBit5;
impl DisableBit for DisableBit5 { fn bit_number(self) -> u8 { 5 } }
struct DisableBit7;
impl DisableBit for DisableBit7 { fn bit_number(self) -> u8 { 7 } }

#[derive(Copy, Clone)]
struct ApuLength {
    disable: bool,
    id: u8,
    remaining: u8,
}

save_struct!(ApuLength { disable, id, remaining });

impl ApuLength {
    fn new() -> ApuLength {
        ApuLength {
            disable: false,
            id: 0,
            remaining: 0,
        }
    }

    // Channels that support the APU Length follow the same register protocol, *except* that the
    // disable bit may be different.
    fn storeb<DB:DisableBit>(&mut self, addr: u16, val: u8, db: DB) {
        match addr & 0x3 {
            0 => self.disable = ((val >> db.bit_number() as usize) & 1) != 0,
            1 | 2 => {}
            3 => {
                // FIXME: Only set `remaining` if APUSTATUS has enabled this channel.
                self.id = val >> 3;
                self.remaining = LENGTH_COUNTERS[self.id as usize];
            }
            _ => panic!("can't happen"),
        }
    }

    fn decrement(&mut self) {
        if self.remaining > 0 && !self.disable {
            self.remaining -= 1;
        }
    }
}

/// Volume envelope
#[derive(Copy, Clone)]
struct ApuEnvelope {
    enabled: bool,
    volume: u8,
    period: u8,
    counter: u8,
    length: ApuLength,
}

save_struct!(ApuEnvelope { enabled, volume, period, counter, length });

impl ApuEnvelope {
    fn new() -> ApuEnvelope {
        ApuEnvelope {
            enabled: false,
            volume: 0,
            period: 0,
            counter: 0,
            length: ApuLength::new(),
        }
    }

    // Channels that support the APU Envelope follow the same register protocol.
    fn storeb(&mut self, addr: u16, val: u8) {
        self.length.storeb(addr, val, DisableBit5);

        if (addr & 0x3) == 0 {
            self.enabled = ((val >> 4) & 1) == 0;
            if self.enabled {
                self.volume = 15;
                self.period = val & 0xf;
                self.counter = 0;
            } else {
                self.volume = val & 0xf;
            }
        }
    }

    // This routine executes at 240 Hz and adjusts the volume and counter appropriately.
    fn tick(&mut self) {
        if self.enabled {
            self.counter += 1;
            if self.counter >= self.period {
                self.counter = 0;
                if self.volume == 0 {
                    if self.loops() {
                        self.volume = 15;
                    }
                } else {
                    self.volume -= 1;
                    if self.volume == 0 && !self.loops() {
                        self.length.remaining = 0;
                    }
                }
            }
        }
    }

    fn loops(self) -> bool {
        self.length.disable
    }

    fn audible(&self) -> bool {
        self.volume > 0 && self.length.remaining > 0
    }

    fn sample_volume(&self) -> i16 {
        (self.volume as i16 * 4) << 8
    }
}

/// Audio frequencies, shared by the pulses and the triangle
#[derive(Copy, Clone)]
struct ApuTimer {
    /// The raw timer value as written to the register.
    value: u16,
    /// How many clock ticks have passed since the last period.
    wavelen_count: u64,
}

save_struct!(ApuTimer { value, wavelen_count });

impl ApuTimer {
    fn new() -> ApuTimer {
        ApuTimer {
            value: 0,
            wavelen_count: 0,
        }
    }

    // Channels that support the APU Envelope follow the same register protocol.
    fn storeb(&mut self, addr: u16, val: u8) {
        match addr & 0x3 {
            0 | 1 => {}
            2 => self.value = (self.value & 0xff00) | (val as u16),
            3 => self.value = (self.value & 0x00ff) | ((val as u16 & 0x7) << 8),
            _ => panic!("can't happen"),
        }
    }

    fn audible(&self) -> bool { self.value > 0 }
    fn wavelen(&self) -> u64 { (self.value as u64 + 1) * 2 }
}

/// APUPULSE: [0x4000, 0x4008)
#[derive(Copy, Clone)]
struct ApuPulse {
    envelope: ApuEnvelope,
    sweep: ApuPulseSweep,
    timer: ApuTimer,
    duty: u8,
    sweep_cycle: u8,
    waveform_index: u8,
}

impl ApuPulse {
    fn new() -> ApuPulse {
        ApuPulse {
            envelope: ApuEnvelope::new(),
            sweep: ApuPulseSweep(0),
            timer: ApuTimer::new(),
            duty: 0,
            sweep_cycle: 0,
            waveform_index: 0,
        }
    }
}

save_struct!(ApuPulse { envelope, sweep, timer, duty, sweep_cycle, waveform_index });

/// APU pulse sweep
#[derive(Copy, Clone)]
struct ApuPulseSweep(u8);

impl Deref for ApuPulseSweep {
    type Target = u8;

    fn deref(&self) -> &u8 {
        &self.0
    }
}

impl DerefMut for ApuPulseSweep {
    fn deref_mut(&mut self) -> &mut u8 {
        &mut self.0
    }
}

impl ApuPulseSweep {
    fn enabled(self) -> bool {
        self.0 >> 7 != 0
    }

    fn period(self) -> u8 {
        ((self.0 >> 4) & 0x7) + 1
    }

    fn negate(self) -> bool {
        ((self.0 >> 3) & 0x1) != 0
    }

    fn shift_count(self) -> u8 {
        self.0 & 0x7
    }
}

/// APUTRIANGLE: [0x4008, 0x400c)
#[derive(Copy, Clone)]
struct ApuTriangle {
    timer: ApuTimer,
    length: ApuLength,
    linear_counter: u8,
    linear_counter_reload: u8,
    linear_counter_halt: bool,
    waveform_index: u8,
}

save_struct!(ApuTriangle { timer, length, linear_counter });

impl ApuTriangle {
    fn new() -> ApuTriangle {
        ApuTriangle {
            timer: ApuTimer::new(),
            length: ApuLength::new(),
            linear_counter: 0,
            linear_counter_reload: 0,
            linear_counter_halt: false,
            waveform_index: 0,
        }
    }

    fn storeb(&mut self, addr: u16, val: u8) {
        self.timer.storeb(addr, val);
        self.length.storeb(addr, val, DisableBit7);

        if (addr & 3) == 0 {
            self.linear_counter_reload = val & 0x7f;
            //self.linear_counter = self.linear_counter_reload;
            self.linear_counter_halt = true;
        }
    }

    // Updates the linear counter. Runs at 240 Hz.
    fn tick(&mut self) {
        if self.linear_counter_halt {
            self.linear_counter = self.linear_counter_reload;
        } else if self.linear_counter != 0 {
            self.linear_counter -= 1;
        }

        if !self.length.disable {
            self.linear_counter_halt = false;
        }
    }

    fn audible(&self) -> bool {
        self.length.remaining > 0 && self.linear_counter > 0
    }
}

/// APUNOISE: [0x400c, 0x4010)
#[derive(Copy, Clone)]
struct ApuNoise {
    envelope: ApuEnvelope,
    /// The number of ticks per possible waveform change.
    timer: u16,
    /// The number of ticks since the last timer.
    timer_count: u16,
    /// The xorshift RNG.
    rng: Xorshift,      // FIXME: This is inaccurate.
}

save_struct!(ApuNoise { envelope, timer, timer_count });

impl ApuNoise {
    fn new() -> ApuNoise {
        ApuNoise {
            envelope: ApuEnvelope::new(),
            timer: 0,
            timer_count: 0,
            rng: Xorshift::new(),
        }
    }
}

/// APUSTATUS: 0x4015
#[derive(Copy, Clone)]
struct ApuStatus(u8);

impl Deref for ApuStatus {
    type Target = u8;

    fn deref(&self) -> &u8 {
        &self.0
    }
}

impl DerefMut for ApuStatus {
    fn deref_mut(&mut self) -> &mut u8 {
        &mut self.0
    }
}

impl ApuStatus {
    fn pulse_enabled(self, channel: u8) -> bool {
        (self.0 >> channel as usize) & 1 != 0
    }

    fn triangle_enabled(self) -> bool {
        self.0 & 0x04 != 0
    }

    fn noise_enabled(self) -> bool {
        self.0 & 0x08 != 0
    }
}

/// Audio registers
#[derive(Copy, Clone)]
struct Regs {
    pulses: [ApuPulse; 2],
    triangle: ApuTriangle,
    noise: ApuNoise,
    status: ApuStatus,
}

impl Save for Regs {
    fn save(&mut self, fd: &mut File) {
        self.pulses[0].save(fd);
        self.pulses[1].save(fd);
        self.triangle.save(fd);
        self.noise.save(fd);
        self.status.save(fd);
    }
    fn load(&mut self, fd: &mut File) {
        self.pulses[0].load(fd);
        self.pulses[1].load(fd);
        self.triangle.load(fd);
        self.noise.load(fd);
        self.status.load(fd);
    }
}

//
// Sample buffers
//

const SAMPLE_COUNT: usize = 178992;

struct SampleBuffer {
    samples: [i16; SAMPLE_COUNT],
}

/// APU state
pub struct Apu {
    regs: Regs,

    sample_buffers: Box<[SampleBuffer; 5]>,
    sample_buffer_offset: usize,
    output_buffer: Option<*mut OutputBuffer>,
    resampler: Resampler,

    pub cy: u64,
    pub ticks: u64,
}

save_struct!(Apu { regs, cy, ticks });

impl Mem for Apu {
    fn loadb(&mut self, addr: u16) -> u8 {
        match addr {
            0x4015 => *self.regs.status,
            _ => 0
        }
    }
    fn storeb(&mut self, addr: u16, val: u8) {
        match addr {
            0x4000 ... 0x4003 => self.update_pulse(addr, val, 0),
            0x4004 ... 0x4007 => self.update_pulse(addr, val, 1),
            0x4008 ... 0x400b => self.regs.triangle.storeb(addr, val),
            0x400c ... 0x400f => self.update_noise(addr, val),
            0x4015 => self.update_status(val),
            _ => {} // TODO
        }
    }
}

impl Apu {
    pub fn new(output_buffer: Option<*mut OutputBuffer>) -> Apu {
        Apu {
            regs: Regs {
                pulses: [ ApuPulse::new(), ApuPulse::new() ],
                triangle: ApuTriangle::new(),
                noise: ApuNoise::new(),
                status: ApuStatus(0),
            },

            sample_buffers: Box::new([
                SampleBuffer {
                    samples: [ 0; SAMPLE_COUNT ]
                },
                SampleBuffer {
                    samples: [ 0; SAMPLE_COUNT ]
                },
                SampleBuffer {
                    samples: [ 0; SAMPLE_COUNT ]
                },
                SampleBuffer {
                    samples: [ 0; SAMPLE_COUNT ]
                },
                SampleBuffer {
                    samples: [ 0; SAMPLE_COUNT ]
                },
            ]),

            sample_buffer_offset: 0,
            output_buffer: output_buffer,
            resampler: Resampler::new(1, NES_SAMPLE_RATE, OUTPUT_SAMPLE_RATE, 0).unwrap(),

            cy: 0,
            ticks: 0,
        }
    }

    fn update_status(&mut self, val: u8) {
        self.regs.status = ApuStatus(val);

        for i in 0..2 {
            if !self.regs.status.pulse_enabled(i as u8) {
                self.regs.pulses[i].envelope.length.remaining = 0;
            }
        }
        if !self.regs.status.triangle_enabled() {
            self.regs.triangle.length.remaining = 0;
        }
        if !self.regs.status.noise_enabled() {
            self.regs.noise.envelope.length.remaining = 0;
        }
    }

    // FIXME: Refactor into a method on ApuPulse itself.
    fn update_pulse(&mut self, addr: u16, val: u8, pulse_number: usize) {
        let pulse = &mut self.regs.pulses[pulse_number];
        pulse.envelope.storeb(addr, val);   // Write to the envelope.
        pulse.timer.storeb(addr, val);      // Write to the timer.
        match addr & 0x3 {
            0 => pulse.duty = val >> 6,
            1 => {
                // TODO: Set reload flag.
                pulse.sweep = ApuPulseSweep(val);
                pulse.sweep_cycle = 0;
            }
            2 | 3 => {}
            _ => panic!("can't happen"),
        }
    }

    // FIXME: Refactor into a method on ApuNoise itself.
    fn update_noise(&mut self, addr: u16, val: u8) {
        self.regs.noise.envelope.storeb(addr, val);

        if (addr & 3) == 2 {
            // TODO: Mode bit.
            self.regs.noise.timer = NOISE_PERIODS[val as usize & 0xf];
        }
    }

    //
    // Playback
    //

    pub fn step(&mut self, run_to_cycle: u64) {
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
            for i in 0..2 {
                let pulse = &mut self.regs.pulses[i];

                // Length counter.
                pulse.envelope.length.decrement();

                // Sweep.
                pulse.sweep_cycle += 1;
                if pulse.sweep_cycle >= pulse.sweep.period() {
                    pulse.sweep_cycle = 0;

                    if pulse.sweep.enabled() {
                        let delta = pulse.timer.value >> pulse.sweep.shift_count() as usize;
                        if !pulse.sweep.negate() {
                            pulse.timer.value += delta;
                        } else {
                            pulse.timer.value -= delta;
                        }
                    }
                }
            }

            // Length counter for triangle and noise.
            self.regs.triangle.length.decrement();
            self.regs.noise.envelope.length.decrement();
        }

        // 240 Hz operations: envelope and linear counter.
        self.regs.pulses[0].envelope.tick();
        self.regs.pulses[1].envelope.tick();
        self.regs.triangle.tick();
        self.regs.noise.envelope.tick();

        // Fill the sample buffers.
        self.play_pulse(0, 0);
        self.play_pulse(1, 1);
        self.play_triangle(2);
        self.play_noise(3);
        self.sample_buffer_offset += NES_SAMPLES_PER_TICK as usize;

        // TODO: 60 Hz IRQ.

        self.ticks += 1;
    }

    //
    // Channel playback
    //

    fn get_or_zero_sample_buffer(buffer: &mut [i16], offset: usize, audible: bool)
                                 -> Option<&mut [i16]> {
        let buffer = &mut buffer[offset..offset + NES_SAMPLES_PER_TICK as usize];
        if audible {
            return Some(buffer);
        }

        for dest in buffer.iter_mut() {
            *dest = 0;
        }
        None
    }

    fn play_pulse(&mut self, pulse_number: usize, channel: usize) {
        let pulse = &mut self.regs.pulses[pulse_number];
        let audible = pulse.envelope.audible() && pulse.timer.audible();
        let buffer_opt = Apu::get_or_zero_sample_buffer(&mut self.sample_buffers[channel].samples,
                                                        self.sample_buffer_offset,
                                                        audible);
        match buffer_opt {
            None => {}
            Some(buffer) => {
                // Process sound.
                // TODO: Vectorize this for speed.
                let volume = pulse.envelope.sample_volume();
                let wavelen = pulse.timer.wavelen();
                let waveform = PULSE_WAVEFORMS[pulse.duty as usize];
                let mut waveform_index = pulse.waveform_index;
                let mut wavelen_count = pulse.timer.wavelen_count;

                for dest in buffer.iter_mut() {
                    wavelen_count += 1;
                    if wavelen_count >= wavelen {
                        wavelen_count = 0;
                        waveform_index = (waveform_index + 1) % 8;
                    }

                    *dest = if ((waveform >> (7 - waveform_index) as usize) & 1) != 0 {
                        volume
                    } else {
                        0
                    };
                }

                pulse.waveform_index = waveform_index;
                pulse.timer.wavelen_count = wavelen_count;
            }
        }
    }

    fn play_triangle(&mut self, channel: usize) {
        let triangle = &mut self.regs.triangle;
        let buffer_opt = Apu::get_or_zero_sample_buffer(&mut self.sample_buffers[channel].samples,
                                                        self.sample_buffer_offset,
                                                        triangle.audible());
        match buffer_opt {
            None => {}
            Some(buffer) => {
                let wavelen = triangle.timer.wavelen() / 2;
                let mut waveform_index = triangle.waveform_index;
                let mut wavelen_count = triangle.timer.wavelen_count;

                for dest in buffer.iter_mut() {
                    wavelen_count += 1;
                    if wavelen_count >= wavelen {
                        wavelen_count = 0;
                        waveform_index = (waveform_index + 1) % 32;
                    }

                    // FIXME: Factor out this calculation.
                    *dest = (TRIANGLE_WAVEFORM[waveform_index as usize] as i16 * 4) << 8;
                }

                triangle.waveform_index = waveform_index;
                triangle.timer.wavelen_count = wavelen_count;
            }
        }
    }

    fn play_noise(&mut self, channel: usize) {
        let noise = &mut self.regs.noise;
        let buffer_opt = Apu::get_or_zero_sample_buffer(&mut self.sample_buffers[channel].samples,
                                                        self.sample_buffer_offset,
                                                        noise.envelope.audible());
        match buffer_opt {
            None => {}
            Some(buffer) => {
                let volume = noise.envelope.sample_volume();
                let timer = noise.timer;
                let mut timer_count = noise.timer_count;
                let mut rng = noise.rng;
                let mut on = 1;

                for dest in buffer.iter_mut() {
                    timer_count += 1;
                    if timer_count >= timer {
                        timer_count = 0;
                        on = rng.next() & 1;
                    }

                    *dest = if on == 0 { 0 } else { volume };
                }

                noise.timer_count = timer_count;
                noise.rng = rng;
            }
        }
    }

    // Resamples and flushes channel buffers to the audio output device if necessary.
    pub fn play_channels(&mut self) {
        let sample_buffer_length = self.sample_buffers[0].samples.len();
        if self.sample_buffer_offset < sample_buffer_length {
            return;
        }
        self.sample_buffer_offset = 0;

        // First, mix all sample buffers into the first one.
        //
        // FIXME: This should not be a linear mix, for accuracy.
        for i in 0..self.sample_buffers[0].samples.len() {
            let mut val = 0;
            for j in 0..5 {
                val += self.sample_buffers[j].samples[i] as i32;
            }

            if val > 32767 {
                val = 32767;
            } else if val < -32768 {
                val = -32768;
            }

            self.sample_buffers[0].samples[i] = val as i16;
        }

        if self.output_buffer.is_none() {
            return;
        }
        let output_buffer = self.output_buffer.unwrap();

        // Wait for the audio callback to catch up if necessary.
        loop {
            unsafe {
                let lock = audio::g_mutex.lock().unwrap();
                let _lock = audio::g_condvar.wait(lock).unwrap();
                if (*output_buffer).play_offset == (*output_buffer).samples.len() {
                    break
                }
            }
        }
        let _lock = audio::lock();
        unsafe {
            // Resample and output the audio.
            let _ = self.resampler.process(0,
                                           &mut self.sample_buffers[0].samples,
                                           &mut (*output_buffer).samples);
            (*output_buffer).play_offset = 0;
        }
    }
}
