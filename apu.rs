//
// sprocketnes/apu.rs
//
// Author: Patrick Walton
//

use mem::Mem;

use core::libc::c_int;
use sdl::mixer::Chunk;
use sdl::mixer;

const PULSE_WAVEFORMS: [[u8 * 8] * 4] = [
    [ 0, 1, 0, 0, 0, 0, 0, 0 ],
    [ 0, 1, 1, 0, 0, 0, 0, 0 ],
    [ 0, 1, 1, 1, 1, 0, 0, 0 ],
    [ 1, 0, 0, 1, 1, 1, 1, 1 ],
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

// TODO
struct ApuPulseSweep;

struct ApuPulse {
    duty: u8,
    envelope_loop: bool,
    volume: ApuPulseVolume,
    sweep: ApuPulseSweep,
    timer: u16,
    // TODO: Length?
}

struct ApuStatus(u8);

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
}

impl Mem for Apu {
    fn loadb(&mut self, _: u16) -> u8 {
        0   // TODO
    }
    fn storeb(&mut self, addr: u16, val: u8) {
        match addr {
            0x4000..0x4003 => self.update_pulse(addr, val, 0),
            0x4004..0x4007 => self.update_pulse(addr, val, 1),
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
                        sweep: ApuPulseSweep,
                        timer: 0
                    }, ..2
                ],
                status: ApuStatus(0),
            },
            chunks: [ c(), c(), c(), c(), c(), ]
        }
    }

    fn close(&mut self) {
        mixer::close();
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
                // TODO: Sweep
            }
            2 => pulse.timer = (pulse.timer & 0xff00) | (val as u16),
            3 => pulse.timer = (pulse.timer & 0x00ff) | ((val as u16 & 0x7) << 8),
            _ => fail!(~"can't happen"),
        }
    }

    //
    // Playback
    //

    fn step(&mut self) {
        self.play_pulse(0, 0);
        self.play_pulse(1, 1);
    }

    fn play_pulse(&mut self, pulse_number: uint, channel: c_int) {
        if mixer::playing(Some(channel)) {
            return;
        }

        let timer = self.regs.pulses[pulse_number].timer as uint;
        if timer == 0 {
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

