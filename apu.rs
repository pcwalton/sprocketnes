//
// sprocketnes/apu.rs
//
// Author: Patrick Walton
//

use mem::Mem;

use core::libc::c_int;
use sdl::mixer::Chunk;
use sdl::mixer;

const CYCLES_PER_TICK: u64 = 7445;

const PULSE_WAVEFORMS: [[u8 * 8] * 4] = [
    [ 0, 1, 0, 0, 0, 0, 0, 0 ],
    [ 0, 1, 1, 0, 0, 0, 0, 0 ],
    [ 0, 1, 1, 1, 1, 0, 0, 0 ],
    [ 1, 0, 0, 1, 1, 1, 1, 1 ],
];

const LENGTH_COUNTERS: [u8 * 32] = [
    10, 254, 20,  2, 40,  4, 80,  6, 160,  8, 60, 10, 14, 12, 26, 14,
    12,  16, 24, 18, 48, 20, 96, 22, 192, 24, 72, 26, 16, 28, 32, 30,
];

//
// APUPULSE: [0x4000, 0x4008)
//

enum ApuPulseVolume {
    ConstantVolume(u8),
    Envelope(u8),
}

impl ApuPulseVolume {
    // TODO: Wrong.
    fn get(self) -> u8 {
        match self {
            ConstantVolume(v) => v,
            Envelope(v) => v,
        }
    }
}

struct ApuPulseSweep(u8);

impl ApuPulseSweep {
    fn enabled(self) -> bool   { (*self >> 7) != 0        }
    fn period(self) -> u8      { ((*self >> 4) & 0x7) + 1 }
    fn negate(self) -> bool    { (*self >> 3) != 0        }
    fn shift_count(self) -> u8 { *self & 0x7              }
}

struct ApuPulse {
    duty: u8,
    envelope_loop: bool,
    volume: ApuPulseVolume,
    sweep: ApuPulseSweep,
    timer: u16,

    length_id: u8,
    length_left: u8,
    sweep_cycle: u8,
}

struct ApuStatus(u8);

impl ApuStatus {
    fn pulse_enabled(self, channel: u8) -> bool { ((*self >> channel) & 1) != 0 }
}

struct Regs {
    pulses: [ApuPulse * 2],
    status: ApuStatus,  // $4015: APUSTATUS
}

//
// General operation
//

pub struct Apu {
    regs: Regs,
    chunks: [Chunk * 5],
    cy: u64,
    ticks: u64,
}

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
    static pub fn new() -> Apu {
        let c = || Chunk::new(vec::from_elem(32768, 0), 127);

        Apu {
            regs: Regs {
                pulses: [
                    ApuPulse {
                        duty: 0,
                        envelope_loop: false,
                        volume: ConstantVolume(0),
                        sweep: ApuPulseSweep(0),
                        timer: 0,
                        length_id: 0,
                        length_left: 0,
                        sweep_cycle: 0,
                    }, ..2
                ],
                status: ApuStatus(0),
            },
            chunks: [ c(), c(), c(), c(), c(), ],
            cy: 0,
            ticks: 0,
        }
    }

    fn close(&mut self) {
        mixer::close();
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
                pulse.envelope_loop = ((val >> 5) & 1) as bool;
                pulse.volume = if (val >> 4 & 1) == 1 {
                    ConstantVolume(val & 0xf)
                } else {
                    Envelope(val & 0xf)
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
            let next_tick_cycle = self.cy + CYCLES_PER_TICK;
            if next_tick_cycle > run_to_cycle {
                break;
            }

            self.tick();
            self.cy += CYCLES_PER_TICK;
        }

        self.play_pulse(0, 0);
        self.play_pulse(1, 1);
    }

    fn tick(&mut self) {
        // 120 Hz operations: length counter and sweep.
        if self.ticks % 2 == 0 {
            // TODO: Remember that triangle wave has a different length disable bit.
            for uint::range(0, 2) |i| {
                let pulse = &mut self.regs.pulses[i];

                // Length counter.
                if pulse.length_left > 0 && !pulse.envelope_loop {
                    // Envelope loop is overlapped with the length disable bit.
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

        self.ticks += 1;
    }

    fn play_pulse(&mut self, pulse_number: uint, channel: c_int) {
        if mixer::playing(Some(channel)) {
            return;
        }

        let timer = self.regs.pulses[pulse_number].timer as uint;
        if timer == 0 {
            return;
        }

        if self.regs.pulses[pulse_number].volume.get() == 0 {
            return;
        }

        if self.regs.pulses[pulse_number].length_left == 0 {
            return;
        }

        let wavelen = (self.regs.pulses[pulse_number].timer as uint + 1) * 2;
        let waveform: &([u8 * 8]) = &PULSE_WAVEFORMS[self.regs.pulses[pulse_number].duty];
        
        // Fill the buffer.
        {
            let buffer = &mut self.chunks[channel].buffer;
            let mut wavelen_count = 0;
            let mut waveform_index = 0;
            for uint::range(0, buffer.len()) |i| {
                if waveform[waveform_index] != 0 {
                    let val = (self.regs.pulses[pulse_number].volume.get() * 4) as u8;
                    buffer[i] = val;
                } else {
                    buffer[i] = 0;
                }

                wavelen_count += 1;
                if wavelen_count == wavelen {
                    wavelen_count = 0;
                    waveform_index = (waveform_index + 1) % 8;
                }
            }
        }

        // Play the buffer.
        self.chunks[channel].play(None, channel);
    }
}

