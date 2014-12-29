//
// sprocketnes/cpu.rs
//
// Author: Patrick Walton
//

use mem::{Mem, MemUtil};
use util::Save;

use libc::{int8_t, int32_t, uint8_t, uint16_t, uint32_t, uint64_t};
use std::io::File;

#[cfg(cpuspew)]
use disasm::Disassembler;

//
// Constants
//

static CARRY_FLAG:    uint8_t = 1 << 0;
static ZERO_FLAG:     uint8_t = 1 << 1;
static IRQ_FLAG:      uint8_t = 1 << 2;
static DECIMAL_FLAG:  uint8_t = 1 << 3;
static BREAK_FLAG:    uint8_t = 1 << 4;
static OVERFLOW_FLAG: uint8_t = 1 << 6;
static NEGATIVE_FLAG: uint8_t = 1 << 7;

static NMI_VECTOR:   uint16_t = 0xfffa;
static RESET_VECTOR: uint16_t = 0xfffc;
static BRK_VECTOR:   uint16_t = 0xfffe;

/// The number of cycles that each machine operation takes. Indexed by opcode number.
///
/// FIXME: This is copied from FCEU.

static CYCLE_TABLE: [uint8_t, ..256] = [
    /*0x00*/ 7,6,2,8,3,3,5,5,3,2,2,2,4,4,6,6,
    /*0x10*/ 2,5,2,8,4,4,6,6,2,4,2,7,4,4,7,7,
    /*0x20*/ 6,6,2,8,3,3,5,5,4,2,2,2,4,4,6,6,
    /*0x30*/ 2,5,2,8,4,4,6,6,2,4,2,7,4,4,7,7,
    /*0x40*/ 6,6,2,8,3,3,5,5,3,2,2,2,3,4,6,6,
    /*0x50*/ 2,5,2,8,4,4,6,6,2,4,2,7,4,4,7,7,
    /*0x60*/ 6,6,2,8,3,3,5,5,4,2,2,2,5,4,6,6,
    /*0x70*/ 2,5,2,8,4,4,6,6,2,4,2,7,4,4,7,7,
    /*0x80*/ 2,6,2,6,3,3,3,3,2,2,2,2,4,4,4,4,
    /*0x90*/ 2,6,2,6,4,4,4,4,2,5,2,5,5,5,5,5,
    /*0xA0*/ 2,6,2,6,3,3,3,3,2,2,2,2,4,4,4,4,
    /*0xB0*/ 2,5,2,5,4,4,4,4,2,4,2,4,4,4,4,4,
    /*0xC0*/ 2,6,2,8,3,3,5,5,2,2,2,2,4,4,6,6,
    /*0xD0*/ 2,5,2,8,4,4,6,6,2,4,2,7,4,4,7,7,
    /*0xE0*/ 2,6,3,8,3,3,5,5,2,2,2,2,4,4,6,6,
    /*0xF0*/ 2,5,2,8,4,4,6,6,2,4,2,7,4,4,7,7,
];

//
// Registers
//

struct Regs {
    a: uint8_t,
    x: uint8_t,
    y: uint8_t,
    s: uint8_t,
    flags: uint8_t,
    pc: uint16_t
}

save_struct!(Regs { a, x, y, s, flags, pc });

impl Regs {
    fn new() -> Regs { Regs { a: 0, x: 0, y: 0, s: 0xfd, flags: 0x24, pc: 0xc000 } }
}

//
// Addressing modes
//

trait AddressingMode<M> {
    fn load(&self, cpu: &mut Cpu<M>) -> uint8_t;
    fn store(&self, cpu: &mut Cpu<M>, val: uint8_t);
}

struct AccumulatorAddressingMode;
impl<M> AddressingMode<M> for AccumulatorAddressingMode where M: Mem {
    fn load(&self, cpu: &mut Cpu<M>) -> uint8_t { cpu.regs.a }
    fn store(&self, cpu: &mut Cpu<M>, val: uint8_t) { cpu.regs.a = val }
}

struct ImmediateAddressingMode;
impl<M> AddressingMode<M> for ImmediateAddressingMode where M: Mem {
    fn load(&self, cpu: &mut Cpu<M>) -> uint8_t { cpu.loadb_bump_pc() }
    fn store(&self, _: &mut Cpu<M>, _: uint8_t) {
        // Not particularly type-safe, but probably not worth using trait inheritance for this.
        panic!("can't store to immediate")
    }
}

struct MemoryAddressingMode{val: uint16_t}

impl Deref<uint16_t> for MemoryAddressingMode {
    fn deref(&self) -> &uint16_t {
        &self.val
    }
}

impl<M> AddressingMode<M> for MemoryAddressingMode where M: Mem {
    fn load(&self, cpu: &mut Cpu<M>) -> uint8_t { cpu.loadb(**self) }
    fn store(&self, cpu: &mut Cpu<M>, val: uint8_t) { cpu.storeb(**self, val) }
}

//
// Opcode decoding
//
// This is implemented as a macro so that both the disassembler and the emulator can use it.
//

macro_rules! decode_op {
    ($op:expr, $this:ident) => {
        // We try to keep this in the same order as the implementations above.
        // TODO: Use arm macros to fix some of this duplication.
        match $op {
            // Loads
            0xa1 => { let v = $this.indexed_indirect_x(); $this.lda(v) }
            0xa5 => { let v = $this.zero_page(); $this.lda(v) }
            0xa9 => { let v = $this.immediate(); $this.lda(v) }
            0xad => { let v = $this.absolute(); $this.lda(v) }
            0xb1 => { let v = $this.indirect_indexed_y(); $this.lda(v) }
            0xb5 => { let v = $this.zero_page_x(); $this.lda(v) }
            0xb9 => { let v = $this.absolute_y(); $this.lda(v) }
            0xbd => { let v = $this.absolute_x(); $this.lda(v) }

            0xa2 => { let v = $this.immediate(); $this.ldx(v) }
            0xa6 => { let v = $this.zero_page(); $this.ldx(v) }
            0xb6 => { let v = $this.zero_page_y(); $this.ldx(v) }
            0xae => { let v = $this.absolute(); $this.ldx(v) }
            0xbe => { let v = $this.absolute_y(); $this.ldx(v) }

            0xa0 => { let v = $this.immediate(); $this.ldy(v) }
            0xa4 => { let v = $this.zero_page(); $this.ldy(v) }
            0xb4 => { let v = $this.zero_page_x(); $this.ldy(v) }
            0xac => { let v = $this.absolute(); $this.ldy(v) }
            0xbc => { let v = $this.absolute_x(); $this.ldy(v) }

            // Stores
            0x85 => { let v = $this.zero_page(); $this.sta(v) }
            0x95 => { let v = $this.zero_page_x(); $this.sta(v) }
            0x8d => { let v = $this.absolute(); $this.sta(v) }
            0x9d => { let v = $this.absolute_x(); $this.sta(v) }
            0x99 => { let v = $this.absolute_y(); $this.sta(v) }
            0x81 => { let v = $this.indexed_indirect_x(); $this.sta(v) }
            0x91 => { let v = $this.indirect_indexed_y(); $this.sta(v) }

            0x86 => { let v = $this.zero_page(); $this.stx(v) }
            0x96 => { let v = $this.zero_page_y(); $this.stx(v) }
            0x8e => { let v = $this.absolute(); $this.stx(v) }

            0x84 => { let v = $this.zero_page(); $this.sty(v) }
            0x94 => { let v = $this.zero_page_x(); $this.sty(v) }
            0x8c => { let v = $this.absolute(); $this.sty(v) }

            // Arithmetic
            0x69 => { let v = $this.immediate(); $this.adc(v) }
            0x65 => { let v = $this.zero_page(); $this.adc(v) }
            0x75 => { let v = $this.zero_page_x(); $this.adc(v) }
            0x6d => { let v = $this.absolute(); $this.adc(v) }
            0x7d => { let v = $this.absolute_x(); $this.adc(v) }
            0x79 => { let v = $this.absolute_y(); $this.adc(v) }
            0x61 => { let v = $this.indexed_indirect_x(); $this.adc(v) }
            0x71 => { let v = $this.indirect_indexed_y(); $this.adc(v) }

            0xe9 => { let v = $this.immediate(); $this.sbc(v) }
            0xe5 => { let v = $this.zero_page(); $this.sbc(v) }
            0xf5 => { let v = $this.zero_page_x(); $this.sbc(v) }
            0xed => { let v = $this.absolute(); $this.sbc(v) }
            0xfd => { let v = $this.absolute_x(); $this.sbc(v) }
            0xf9 => { let v = $this.absolute_y(); $this.sbc(v) }
            0xe1 => { let v = $this.indexed_indirect_x(); $this.sbc(v) }
            0xf1 => { let v = $this.indirect_indexed_y(); $this.sbc(v) }

            // Comparisons
            0xc9 => { let v = $this.immediate(); $this.cmp(v) }
            0xc5 => { let v = $this.zero_page(); $this.cmp(v) }
            0xd5 => { let v = $this.zero_page_x(); $this.cmp(v) }
            0xcd => { let v = $this.absolute(); $this.cmp(v) }
            0xdd => { let v = $this.absolute_x(); $this.cmp(v) }
            0xd9 => { let v = $this.absolute_y(); $this.cmp(v) }
            0xc1 => { let v = $this.indexed_indirect_x(); $this.cmp(v) }
            0xd1 => { let v = $this.indirect_indexed_y(); $this.cmp(v) }

            0xe0 => { let v = $this.immediate(); $this.cpx(v) }
            0xe4 => { let v = $this.zero_page(); $this.cpx(v) }
            0xec => { let v = $this.absolute(); $this.cpx(v) }

            0xc0 => { let v = $this.immediate(); $this.cpy(v) }
            0xc4 => { let v = $this.zero_page(); $this.cpy(v) }
            0xcc => { let v = $this.absolute(); $this.cpy(v) }

            // Bitwise operations
            0x29 => { let v = $this.immediate(); $this.and(v) }
            0x25 => { let v = $this.zero_page(); $this.and(v) }
            0x35 => { let v = $this.zero_page_x(); $this.and(v) }
            0x2d => { let v = $this.absolute(); $this.and(v) }
            0x3d => { let v = $this.absolute_x(); $this.and(v) }
            0x39 => { let v = $this.absolute_y(); $this.and(v) }
            0x21 => { let v = $this.indexed_indirect_x(); $this.and(v) }
            0x31 => { let v = $this.indirect_indexed_y(); $this.and(v) }

            0x09 => { let v = $this.immediate(); $this.ora(v) }
            0x05 => { let v = $this.zero_page(); $this.ora(v) }
            0x15 => { let v = $this.zero_page_x(); $this.ora(v) }
            0x0d => { let v = $this.absolute(); $this.ora(v) }
            0x1d => { let v = $this.absolute_x(); $this.ora(v) }
            0x19 => { let v = $this.absolute_y(); $this.ora(v) }
            0x01 => { let v = $this.indexed_indirect_x(); $this.ora(v) }
            0x11 => { let v = $this.indirect_indexed_y(); $this.ora(v) }

            0x49 => { let v = $this.immediate(); $this.eor(v) }
            0x45 => { let v = $this.zero_page(); $this.eor(v) }
            0x55 => { let v = $this.zero_page_x(); $this.eor(v) }
            0x4d => { let v = $this.absolute(); $this.eor(v) }
            0x5d => { let v = $this.absolute_x(); $this.eor(v) }
            0x59 => { let v = $this.absolute_y(); $this.eor(v) }
            0x41 => { let v = $this.indexed_indirect_x(); $this.eor(v) }
            0x51 => { let v = $this.indirect_indexed_y(); $this.eor(v) }

            0x24 => { let v = $this.zero_page(); $this.bit(v) }
            0x2c => { let v = $this.absolute(); $this.bit(v) }

            // Shifts and rotates
            0x2a => { let v = $this.accumulator(); $this.rol(v) }
            0x26 => { let v = $this.zero_page(); $this.rol(v) }
            0x36 => { let v = $this.zero_page_x(); $this.rol(v) }
            0x2e => { let v = $this.absolute(); $this.rol(v) }
            0x3e => { let v = $this.absolute_x(); $this.rol(v) }

            0x6a => { let v = $this.accumulator(); $this.ror(v) }
            0x66 => { let v = $this.zero_page(); $this.ror(v) }
            0x76 => { let v = $this.zero_page_x(); $this.ror(v) }
            0x6e => { let v = $this.absolute(); $this.ror(v) }
            0x7e => { let v = $this.absolute_x(); $this.ror(v) }

            0x0a => { let v = $this.accumulator(); $this.asl(v) }
            0x06 => { let v = $this.zero_page(); $this.asl(v) }
            0x16 => { let v = $this.zero_page_x(); $this.asl(v) }
            0x0e => { let v = $this.absolute(); $this.asl(v) }
            0x1e => { let v = $this.absolute_x(); $this.asl(v) }

            0x4a => { let v = $this.accumulator(); $this.lsr(v) }
            0x46 => { let v = $this.zero_page(); $this.lsr(v) }
            0x56 => { let v = $this.zero_page_x(); $this.lsr(v) }
            0x4e => { let v = $this.absolute(); $this.lsr(v) }
            0x5e => { let v = $this.absolute_x(); $this.lsr(v) }

            // Increments and decrements
            0xe6 => { let v = $this.zero_page(); $this.inc(v) }
            0xf6 => { let v = $this.zero_page_x(); $this.inc(v) }
            0xee => { let v = $this.absolute(); $this.inc(v) }
            0xfe => { let v = $this.absolute_x(); $this.inc(v) }

            0xc6 => { let v = $this.zero_page(); $this.dec(v) }
            0xd6 => { let v = $this.zero_page_x(); $this.dec(v) }
            0xce => { let v = $this.absolute(); $this.dec(v) }
            0xde => { let v = $this.absolute_x(); $this.dec(v) }

            0xe8 => $this.inx(),
            0xca => $this.dex(),
            0xc8 => $this.iny(),
            0x88 => $this.dey(),

            // Register moves
            0xaa => $this.tax(),
            0xa8 => $this.tay(),
            0x8a => $this.txa(),
            0x98 => $this.tya(),
            0x9a => $this.txs(),
            0xba => $this.tsx(),

            // Flag operations
            0x18 => $this.clc(),
            0x38 => $this.sec(),
            0x58 => $this.cli(),
            0x78 => $this.sei(),
            0xb8 => $this.clv(),
            0xd8 => $this.cld(),
            0xf8 => $this.sed(),

            // Branches
            0x10 => $this.bpl(),
            0x30 => $this.bmi(),
            0x50 => $this.bvc(),
            0x70 => $this.bvs(),
            0x90 => $this.bcc(),
            0xb0 => $this.bcs(),
            0xd0 => $this.bne(),
            0xf0 => $this.beq(),

            // Jumps
            0x4c => $this.jmp(),
            0x6c => $this.jmpi(),

            // Procedure calls
            0x20 => $this.jsr(),
            0x60 => $this.rts(),
            0x00 => $this.brk(),
            0x40 => $this.rti(),

            // Stack operations
            0x48 => $this.pha(),
            0x68 => $this.pla(),
            0x08 => $this.php(),
            0x28 => $this.plp(),

            // No operation
            0xea => $this.nop(),

            _ => panic!("unimplemented or illegal instruction: {}", $op)
        }
    }
}

//
// Main CPU implementation
//

pub type Cycles = uint64_t;

/// The main CPU structure definition.
pub struct Cpu<M> {
    pub cy: Cycles,
    regs: Regs,
    pub mem: M,
}

// The CPU implements Mem so that it can handle writes to the DMA register.
impl<M> Mem for Cpu<M> where M: Mem {
    fn loadb(&mut self, addr: uint16_t) -> uint8_t { self.mem.loadb(addr) }
    fn storeb(&mut self, addr: uint16_t, val: uint8_t) {
        // Handle OAM_DMA.
        if addr == 0x4014 {
            self.dma(val)
        } else {
            self.mem.storeb(addr, val)
        }
    }
}

// Save state logic.
impl<M> Save for Cpu<M> where M: Mem + Save {
    fn save(&mut self, fd: &mut File) {
        self.cy.save(fd);
        self.regs.save(fd);
        self.mem.save(fd);
    }
    fn load(&mut self, fd: &mut File) {
        self.cy.load(fd);
        self.regs.load(fd);
        self.mem.load(fd);
    }
}

impl<M> Cpu<M> where M: Mem {
    // Debugging
    #[cfg(cpuspew)]
    fn trace(&mut self) {
        let mut disassembler = Disassembler {
            pc: self.regs.pc,
            mem: &mut self.mem
        };
        println!(
            "{:04X} {:20s} A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} CYC:{}",
            self.regs.pc as uint,
            disassembler.disassemble(),
            self.regs.a as uint,
            self.regs.x as uint,
            self.regs.y as uint,
            self.regs.flags as uint,
            self.regs.s as uint,
            self.cy as uint
        );
    }
    #[cfg(not(cpuspew))]
    fn trace(&mut self) {}

    // Performs DMA to the OAMDATA ($2004) register.
    fn dma(&mut self, hi_addr: uint8_t) {
        for addr in range(hi_addr as uint << 8, (hi_addr + 1) as uint << 8) {
            let val = self.loadb(addr as uint16_t);
            self.storeb(0x2004, val);

            // FIXME: The last address sometimes takes 1 cycle, sometimes 2 -- NESdev isn't very
            // clear on this.
            self.cy += 2;
        }
    }

    // Memory access helpers
    /// Loads the byte at the program counter and increments the program counter.
    fn loadb_bump_pc(&mut self) -> uint8_t {
        let pc = self.regs.pc;
        let val = self.loadb(pc);
        self.regs.pc += 1;
        val
    }
    /// Loads two bytes (little-endian) at the program counter and bumps the program counter over
    /// them.
    fn loadw_bump_pc(&mut self) -> uint16_t {
        let pc = self.regs.pc;
        let val = self.loadw(pc);
        self.regs.pc += 2;
        val
    }

    // Stack helpers
    fn pushb(&mut self, val: uint8_t) {
        let s = self.regs.s;
        self.storeb(0x100 + s as uint16_t, val);
        self.regs.s -= 1;
    }
    fn pushw(&mut self, val: uint16_t) {
        // FIXME: Is this correct? FCEU has two self.storeb()s here. Might have different
        // semantics...
        let s = self.regs.s;
        self.storew(0x100 + (s - 1) as uint16_t, val);
        self.regs.s -= 2;
    }
    fn popb(&mut self) -> uint8_t {
        let s = self.regs.s;
        let val = self.loadb(0x100 + s as uint16_t + 1);
        self.regs.s += 1;
        val
    }
    fn popw(&mut self) -> uint16_t {
        // FIXME: See comment in pushw().
        let s = self.regs.s;
        let val = self.loadw(0x100 + s as uint16_t + 1);
        self.regs.s += 2;
        val
    }

    // Flag helpers
    fn get_flag(&self, flag: uint8_t) -> bool { (self.regs.flags & flag) != 0 }
    fn set_flag(&mut self, flag: uint8_t, on: bool) {
        if on {
            self.regs.flags |= flag;
        } else {
            self.regs.flags &= !flag;
        }
    }
    fn set_flags(&mut self, val: uint8_t) {
        // Flags get munged in a strange way relating to the unused bit 5 on the NES.
        self.regs.flags = (val | 0x30) - 0x10;
    }
    fn set_zn(&mut self, val: uint8_t) -> uint8_t {
        self.set_flag(ZERO_FLAG, val == 0);
        self.set_flag(NEGATIVE_FLAG, (val & 0x80) != 0);
        val
    }

    // Addressing modes
    fn immediate(&mut self) -> ImmediateAddressingMode { ImmediateAddressingMode }
    fn accumulator(&mut self) -> AccumulatorAddressingMode { AccumulatorAddressingMode }
    fn zero_page(&mut self) -> MemoryAddressingMode {
        MemoryAddressingMode{val: self.loadb_bump_pc() as uint16_t}
    }
    fn zero_page_x(&mut self) -> MemoryAddressingMode {
        MemoryAddressingMode{val: (self.loadb_bump_pc() + self.regs.x) as uint16_t}
    }
    fn zero_page_y(&mut self) -> MemoryAddressingMode {
        MemoryAddressingMode{val: (self.loadb_bump_pc() + self.regs.y) as uint16_t}
    }
    fn absolute(&mut self) -> MemoryAddressingMode {
        MemoryAddressingMode{val: self.loadw_bump_pc()}
    }
    fn absolute_x(&mut self) -> MemoryAddressingMode {
        MemoryAddressingMode{val: self.loadw_bump_pc() + self.regs.x as uint16_t}
    }
    fn absolute_y(&mut self) -> MemoryAddressingMode {
        MemoryAddressingMode{val: self.loadw_bump_pc() + self.regs.y as uint16_t}
    }
    fn indexed_indirect_x(&mut self) -> MemoryAddressingMode {
        let val = self.loadb_bump_pc();
        let x = self.regs.x;
        let addr = self.loadw_zp(val + x);
        MemoryAddressingMode{val: addr}
    }
    fn indirect_indexed_y(&mut self) -> MemoryAddressingMode {
        let val = self.loadb_bump_pc();
        let y = self.regs.y;
        let addr = self.loadw_zp(val) + y as uint16_t;
        MemoryAddressingMode{val: addr}
    }

    //
    // Instructions
    //

    // Loads
    fn lda<AM:AddressingMode<M>>(&mut self, am: AM) {
        let val = am.load(self);
        self.regs.a = self.set_zn(val)
    }
    fn ldx<AM:AddressingMode<M>>(&mut self, am: AM) {
        let val = am.load(self);
        self.regs.x = self.set_zn(val)
    }
    fn ldy<AM:AddressingMode<M>>(&mut self, am: AM) {
        let val = am.load(self);
        self.regs.y = self.set_zn(val)
    }

    // Stores
    fn sta<AM:AddressingMode<M>>(&mut self, am: AM) {
        let a = self.regs.a;
        am.store(self, a)
    }
    fn stx<AM:AddressingMode<M>>(&mut self, am: AM) {
        let x = self.regs.x;
        am.store(self, x)
    }
    fn sty<AM:AddressingMode<M>>(&mut self, am: AM) {
        let y = self.regs.y;
        am.store(self, y)
    }

    // Arithmetic
    #[inline(always)]
    fn adc<AM:AddressingMode<M>>(&mut self, am: AM) {
        let val = am.load(self);
        let mut result = self.regs.a as uint32_t + val as uint32_t;
        if self.get_flag(CARRY_FLAG) {
            result += 1;
        }

        self.set_flag(CARRY_FLAG, (result & 0x100) != 0);

        let result = result as uint8_t;
        let a = self.regs.a;
        self.set_flag(OVERFLOW_FLAG, (a ^ val) & 0x80 == 0 && (a ^ result) & 0x80 == 0x80);
        self.regs.a = self.set_zn(result);
    }
    #[inline(always)]
    fn sbc<AM:AddressingMode<M>>(&mut self, am: AM) {
        let val = am.load(self);
        let a = self.regs.a;
        let mut result = a as uint32_t - val as uint32_t;
        if !self.get_flag(CARRY_FLAG) {
            result -= 1;
        }

        self.set_flag(CARRY_FLAG, (result & 0x100) == 0);

        let result = result as uint8_t;
        let a = self.regs.a;
        self.set_flag(OVERFLOW_FLAG, (a ^ result) & 0x80 != 0 && (a ^ val) & 0x80 == 0x80);
        self.regs.a = self.set_zn(result);
    }

    // Comparisons
    fn cmp_base<AM:AddressingMode<M>>(&mut self, x: uint8_t, am: AM) {
        let y = am.load(self);
        let result = x as uint32_t - y as uint32_t;
        self.set_flag(CARRY_FLAG, (result & 0x100) == 0);
        let _ = self.set_zn(result as uint8_t);
    }
    fn cmp<AM:AddressingMode<M>>(&mut self, am: AM) {
        let a = self.regs.a;
        self.cmp_base(a, am)
    }
    fn cpx<AM:AddressingMode<M>>(&mut self, am: AM) {
        let x = self.regs.x;
        self.cmp_base(x, am)
    }
    fn cpy<AM:AddressingMode<M>>(&mut self, am: AM) {
        let y = self.regs.y;
        self.cmp_base(y, am)
    }

    // Bitwise operations
    fn and<AM:AddressingMode<M>>(&mut self, am: AM) {
        let val = am.load(self) & self.regs.a;
        self.regs.a = self.set_zn(val)
    }
    fn ora<AM:AddressingMode<M>>(&mut self, am: AM) {
        let val = am.load(self) | self.regs.a;
        self.regs.a = self.set_zn(val)
    }
    fn eor<AM:AddressingMode<M>>(&mut self, am: AM) {
        let val = am.load(self) ^ self.regs.a;
        self.regs.a = self.set_zn(val)
    }
    fn bit<AM:AddressingMode<M>>(&mut self, am: AM) {
        let val = am.load(self);
        let a = self.regs.a;
        self.set_flag(ZERO_FLAG, (val & a) == 0);
        self.set_flag(NEGATIVE_FLAG, (val & 0x80) != 0);
        self.set_flag(OVERFLOW_FLAG, (val & 0x40) != 0);
    }

    // Shifts and rotates
    fn shl_base<AM:AddressingMode<M>>(&mut self, lsb: bool, am: AM) {
        let val = am.load(self);
        let new_carry = (val & 0x80) != 0;
        let mut result = val << 1;
        if lsb {
            result |= 1;
        }
        self.set_flag(CARRY_FLAG, new_carry);
        let val = self.set_zn(result as uint8_t);
        am.store(self, val)
    }
    fn shr_base<AM:AddressingMode<M>>(&mut self, msb: bool, am: AM) {
        let val = am.load(self);
        let new_carry = (val & 0x1) != 0;
        let mut result = val >> 1;
        if msb {
            result |= 0x80;
        }
        self.set_flag(CARRY_FLAG, new_carry);
        let val = self.set_zn(result as uint8_t);
        am.store(self, val)
    }
    fn rol<AM:AddressingMode<M>>(&mut self, am: AM) {
        let val = self.get_flag(CARRY_FLAG);
        self.shl_base(val, am)
    }
    fn ror<AM:AddressingMode<M>>(&mut self, am: AM) {
        let val = self.get_flag(CARRY_FLAG);
        self.shr_base(val, am)
    }
    fn asl<AM:AddressingMode<M>>(&mut self, am: AM) { self.shl_base(false, am) }
    fn lsr<AM:AddressingMode<M>>(&mut self, am: AM) { self.shr_base(false, am) }

    // Increments and decrements
    fn inc<AM:AddressingMode<M>>(&mut self, am: AM) {
        let val = am.load(self);
        let val = self.set_zn(val + 1);
        am.store(self, val)
    }
    fn dec<AM:AddressingMode<M>>(&mut self, am: AM) {
        let val = am.load(self);
        let val = self.set_zn(val - 1);
        am.store(self, val)
    }
    fn inx(&mut self) {
        let x = self.regs.x;
        self.regs.x = self.set_zn(x + 1)
    }
    fn dex(&mut self) {
        let x = self.regs.x;
        self.regs.x = self.set_zn(x - 1)
    }
    fn iny(&mut self) {
        let y = self.regs.y;
        self.regs.y = self.set_zn(y + 1)
    }
    fn dey(&mut self) {
        let y = self.regs.y;
        self.regs.y = self.set_zn(y - 1)
    }

    // Register moves
    fn tax(&mut self) {
        let a = self.regs.a;
        self.regs.x = self.set_zn(a)
    }
    fn tay(&mut self) {
        let a = self.regs.a;
        self.regs.y = self.set_zn(a)
    }
    fn txa(&mut self) {
        let x = self.regs.x;
        self.regs.a = self.set_zn(x)
    }
    fn tya(&mut self) {
        let y = self.regs.y;
        self.regs.a = self.set_zn(y)
    }
    fn txs(&mut self) { self.regs.s = self.regs.x }
    fn tsx(&mut self) {
        let s = self.regs.s;
        self.regs.x = self.set_zn(s)
    }

    // Flag operations
    fn clc(&mut self) { self.set_flag(CARRY_FLAG, false) }
    fn sec(&mut self) { self.set_flag(CARRY_FLAG, true) }
    fn cli(&mut self) { self.set_flag(IRQ_FLAG, false) }
    fn sei(&mut self) { self.set_flag(IRQ_FLAG, true) }
    fn clv(&mut self) { self.set_flag(OVERFLOW_FLAG, false) }
    fn cld(&mut self) { self.set_flag(DECIMAL_FLAG, false) }
    fn sed(&mut self) { self.set_flag(DECIMAL_FLAG, true) }

    // Branches
    fn bra_base(&mut self, cond: bool) {
        let disp = self.loadb_bump_pc() as int8_t;
        if cond {
            self.regs.pc = (self.regs.pc as int32_t + disp as int32_t) as uint16_t;
        }
    }
    fn bpl(&mut self) {
        let flag = !self.get_flag(NEGATIVE_FLAG);
        self.bra_base(flag)
    }
    fn bmi(&mut self) {
        let flag = self.get_flag(NEGATIVE_FLAG);
        self.bra_base(flag)
    }
    fn bvc(&mut self) {
        let flag = !self.get_flag(OVERFLOW_FLAG);
        self.bra_base(flag)
    }
    fn bvs(&mut self) {
        let flag = self.get_flag(OVERFLOW_FLAG);
        self.bra_base(flag)
    }
    fn bcc(&mut self) {
        let flag = !self.get_flag(CARRY_FLAG);
        self.bra_base(flag)
    }
    fn bcs(&mut self) {
        let flag = self.get_flag(CARRY_FLAG);
        self.bra_base(flag)
    }
    fn bne(&mut self) {
        let flag = !self.get_flag(ZERO_FLAG);
        self.bra_base(flag)
    }
    fn beq(&mut self) {
        let flag = self.get_flag(ZERO_FLAG);
        self.bra_base(flag)
    }

    // Jumps
    fn jmp(&mut self) { self.regs.pc = self.loadw_bump_pc() }
    fn jmpi(&mut self) {
        let addr = self.loadw_bump_pc();

        // Replicate the famous CPU bug...
        let lo = self.loadb(addr);
        let hi = self.loadb((addr & 0xff00) | ((addr + 1) & 0x00ff));

        self.regs.pc = (hi as uint16_t << 8) | lo as uint16_t;
    }

    // Procedure calls
    fn jsr(&mut self) {
        let addr = self.loadw_bump_pc();
        let pc = self.regs.pc;
        self.pushw(pc - 1);
        self.regs.pc = addr;
    }
    fn rts(&mut self) { self.regs.pc = self.popw() + 1 }
    fn brk(&mut self) {
        let pc = self.regs.pc;
        self.pushw(pc + 1);
        let flags = self.regs.flags;
        self.pushb(flags);    // FIXME: FCEU sets BREAK_FLAG and U_FLAG here, why?
        self.set_flag(IRQ_FLAG, true);
        self.regs.pc = self.loadw(BRK_VECTOR);
    }
    fn rti(&mut self) {
        let flags = self.popb();
        self.set_flags(flags);
        self.regs.pc = self.popw(); // NB: no + 1
    }

    // Stack operations
    fn pha(&mut self) {
        let a = self.regs.a;
        self.pushb(a)
    }
    fn pla(&mut self) {
        let val = self.popb();
        self.regs.a = self.set_zn(val)
    }
    fn php(&mut self) {
        let flags = self.regs.flags;
        self.pushb(flags | BREAK_FLAG)
    }
    fn plp(&mut self) {
        let val = self.popb();
        self.set_flags(val)
    }

    // No operation
    fn nop(&mut self) {}

    // The main fetch-and-decode routine
    pub fn step(&mut self) {
        self.trace();

        let op = self.loadb_bump_pc();
        decode_op!(op, self);

        self.cy += CYCLE_TABLE[op as uint] as Cycles;
    }

    /// External interfaces
    pub fn reset(&mut self) { self.regs.pc = self.loadw(RESET_VECTOR); }

    pub fn nmi(&mut self) {
        let (pc, flags) = (self.regs.pc, self.regs.flags);
        self.pushw(pc);
        self.pushb(flags);
        self.regs.pc = self.loadw(NMI_VECTOR);
    }

    pub fn irq(&mut self) {
        if self.get_flag(IRQ_FLAG) {
            return;
        }

        let (pc, flags) = (self.regs.pc, self.regs.flags);
        self.pushw(pc);
        self.pushb(flags);
        self.regs.pc = self.loadw(BRK_VECTOR);
    }

    /// The constructor.
    pub fn new(mem: M) -> Cpu<M> { Cpu { cy: 0, regs: Regs::new(), mem: mem } }
}
