//
// sprocketnes/cpu.rs
//
// Copyright (c) 2012 Mozilla Foundation
// Author: Patrick Walton
//

use disasm::Disassembler;
use util::println;

//
// Constants
//

const CARRY_FLAG:    u8 = 1 << 0;
const ZERO_FLAG:     u8 = 1 << 1;
const IRQ_FLAG:      u8 = 1 << 2;
const DECIMAL_FLAG:  u8 = 1 << 3;
const BREAK_FLAG:    u8 = 1 << 4;
const OVERFLOW_FLAG: u8 = 1 << 6;
const NEGATIVE_FLAG: u8 = 1 << 7;

const RESET_VECTOR: u16 = 0xfffc; 
const BRK_VECTOR:   u16 = 0xfffe;

/// The number of cycles that each machine operation takes. Indexed by opcode number.
///
/// FIXME: This is copied from FCEU.

const CYCLE_TABLE: [u8 * 256] = [
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
// The memory interface
//

/// The basic memory interface
pub trait Mem {
    fn loadb(&mut self, addr: u16) -> u8;
    fn storeb(&mut self, addr: u16, val: u8);

    // These two could be defined in terms of the base, but often (e.g. on x86) it is possible to
    // use unaligned reads/writes to implement these more efficiently.
    fn loadw(&mut self, addr: u16) -> u16;
    fn storew(&mut self, addr: u16, val: u16);
}

//
// Registers
//

struct Regs {
    a: u8,
    x: u8,
    y: u8,
    s: u8,
    flags: u8,
    pc: u16
}

impl Regs {
    static fn new() -> Regs { Regs { a: 0, x: 0, y: 0, s: 0xfd, flags: 0x24, pc: 0xc000 } }
}

//
// Addressing modes
//

trait AddressingMode<M> {
    fn load(&self, cpu: &mut Cpu<M>) -> u8;
    fn store(&self, cpu: &mut Cpu<M>, val: u8);
}

struct AccumulatorAddressingMode;
impl<M:Mem> AccumulatorAddressingMode : AddressingMode<M> {
    fn load(&self, cpu: &mut Cpu<M>) -> u8 { cpu.regs.a }
    fn store(&self, cpu: &mut Cpu<M>, val: u8) { cpu.regs.a = val }
}

struct ImmediateAddressingMode;
impl<M:Mem> ImmediateAddressingMode : AddressingMode<M> {
    fn load(&self, cpu: &mut Cpu<M>) -> u8 { cpu.loadb_bump_pc() }
    fn store(&self, _: &mut Cpu<M>, _: u8) {
        // Not particularly type-safe, but probably not worth using trait inheritance for this.
        fail ~"can't store to immediate"
    }
}

struct MemoryAddressingMode(u16);
impl<M:Mem> MemoryAddressingMode : AddressingMode<M> {
    fn load(&self, cpu: &mut Cpu<M>) -> u8 { cpu.mem.loadb(**self) }
    fn store(&self, cpu: &mut Cpu<M>, val: u8) { cpu.mem.storeb(**self, val) }
}

//
// Opcode decoding
//
// This is implemented as a macro so that both the disassembler and the emulator can use it.
//

macro_rules! decode_op {
    (
        op: $op:expr,
        this: $this:expr,
        modes: [
            $immediate:expr,
            $accumulator:expr,
            $zero_page:expr,
            $zero_page_x:expr,
            $zero_page_y:expr,
            $absolute:expr,
            $absolute_x:expr,
            $absolute_y:expr,
            $indexed_indirect_x:expr,
            $indirect_indexed_y:expr
        ]
    ) => {
        // We try to keep this in the same order as the implementations above.
        // TODO: Use arm macros to fix some of this duplication.
        match $op {
            // Loads
            0xa1 => $this.lda($indexed_indirect_x),
            0xa5 => $this.lda($zero_page),
            0xa9 => $this.lda($immediate),
            0xad => $this.lda($absolute),
            0xb1 => $this.lda($indirect_indexed_y),
            0xb5 => $this.lda($zero_page_x),
            0xb9 => $this.lda($absolute_y),
            0xbd => $this.lda($absolute_x),

            0xa2 => $this.ldx($immediate),
            0xa6 => $this.ldx($zero_page),
            0xae => $this.ldx($zero_page_y),
            0xb6 => $this.ldx($absolute),
            0xbe => $this.ldx($absolute_y),

            0xa0 => $this.ldy($immediate),
            0xa4 => $this.ldy($zero_page),
            0xac => $this.ldy($zero_page_x),
            0xb4 => $this.ldy($absolute),
            0xbc => $this.ldy($absolute_x),

            // Stores
            0x85 => $this.sta($zero_page),
            0x95 => $this.sta($zero_page_x),
            0x8d => $this.sta($absolute),
            0x9d => $this.sta($absolute_x),
            0x99 => $this.sta($absolute_y),
            0x81 => $this.sta($indexed_indirect_x),
            0x91 => $this.sta($indirect_indexed_y),

            0x86 => $this.stx($zero_page),
            0x96 => $this.stx($zero_page_y),
            0x8e => $this.stx($absolute),

            0x84 => $this.sty($zero_page),
            0x94 => $this.sty($zero_page_x),
            0x8c => $this.sty($absolute),

            // Arithmetic
            0x69 => $this.adc($immediate),
            0x65 => $this.adc($zero_page),
            0x75 => $this.adc($zero_page_x),
            0x6d => $this.adc($absolute),
            0x7d => $this.adc($absolute_x),
            0x79 => $this.adc($absolute_y),
            0x61 => $this.adc($indexed_indirect_x),
            0x71 => $this.adc($indirect_indexed_y),

            0xe9 => $this.sbc($immediate),
            0xe5 => $this.sbc($zero_page),
            0xf5 => $this.sbc($zero_page_x),
            0xed => $this.sbc($absolute),
            0xfd => $this.sbc($absolute_x),
            0xf9 => $this.sbc($absolute_y),
            0xe1 => $this.sbc($indexed_indirect_x),
            0xf1 => $this.sbc($indirect_indexed_y),

            // Comparisons
            0xc9 => $this.cmp($immediate),
            0xc5 => $this.cmp($zero_page),
            0xd5 => $this.cmp($zero_page_x),
            0xcd => $this.cmp($absolute),
            0xdd => $this.cmp($absolute_x),
            0xd9 => $this.cmp($absolute_y),
            0xc1 => $this.cmp($indexed_indirect_x),
            0xd1 => $this.cmp($indirect_indexed_y),

            0xe0 => $this.cpx($immediate),
            0xe4 => $this.cpx($zero_page),
            0xec => $this.cpx($absolute),

            0xc0 => $this.cpy($immediate),
            0xc4 => $this.cpy($zero_page),
            0xcc => $this.cpy($absolute),

            // Bitwise operations
            0x29 => $this.and($immediate),
            0x25 => $this.and($zero_page),
            0x35 => $this.and($zero_page_x),
            0x2d => $this.and($absolute),
            0x3d => $this.and($absolute_x),
            0x39 => $this.and($absolute_y),
            0x21 => $this.and($indexed_indirect_x),
            0x31 => $this.and($indirect_indexed_y),

            0x09 => $this.ora($immediate),
            0x05 => $this.ora($zero_page),
            0x15 => $this.ora($zero_page_x),
            0x0d => $this.ora($absolute),
            0x1d => $this.ora($absolute_x),
            0x19 => $this.ora($absolute_y),
            0x01 => $this.ora($indexed_indirect_x),
            0x11 => $this.ora($indirect_indexed_y),

            0x49 => $this.eor($immediate),
            0x45 => $this.eor($zero_page),
            0x55 => $this.eor($zero_page_x),
            0x4d => $this.eor($absolute),
            0x5d => $this.eor($absolute_x),
            0x59 => $this.eor($absolute_y),
            0x41 => $this.eor($indexed_indirect_x),
            0x51 => $this.eor($indirect_indexed_y),

            0x24 => $this.bit($zero_page),
            0x2c => $this.bit($absolute),

            // Shifts and rotates
            0x2a => $this.rol($accumulator),
            0x26 => $this.rol($zero_page),
            0x36 => $this.rol($zero_page_x),
            0x2e => $this.rol($absolute),
            0x3e => $this.rol($absolute_x),

            0x6a => $this.ror($accumulator),
            0x66 => $this.ror($zero_page),
            0x76 => $this.ror($zero_page_x),
            0x6e => $this.ror($absolute),
            0x7e => $this.ror($absolute_x),

            0x0a => $this.asl($accumulator),
            0x06 => $this.asl($zero_page),
            0x16 => $this.asl($zero_page_x),
            0x0e => $this.asl($absolute),
            0x1e => $this.asl($absolute_x),

            0x4a => $this.lsr($accumulator),
            0x46 => $this.lsr($zero_page),
            0x56 => $this.lsr($zero_page_x),
            0x4e => $this.lsr($absolute),
            0x5e => $this.lsr($absolute_x),

            // Increments and decrements
            0xe6 => $this.inc($zero_page),
            0xf6 => $this.inc($zero_page_x),
            0xee => $this.inc($absolute),
            0xfe => $this.inc($absolute_x),

            0xc6 => $this.dec($zero_page),
            0xd6 => $this.dec($zero_page_x),
            0xce => $this.dec($absolute),
            0xde => $this.dec($absolute_x),

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

            _ => fail ~"unimplemented or illegal instruction"
        }
    }
}

//
// Main CPU implementation
//

type Cycles = u64;

/// The main CPU structure definition.
pub struct Cpu<M> {
    cy: Cycles,
    regs: Regs,
    mem: M,
}

// FIXME: This should not need to be public! Sigh. Resolve bug.
pub impl<M:Mem> Cpu<M> {
    // Debugging
    #[cfg(debug)]
    fn trace(&mut self) {
        let mut disassembler = Disassembler { pc: self.regs.pc, mem: &mut self.mem };
        println(fmt!(
            "%04X %-20s A:%02X X:%02X Y:%02X P:%02X SP:%02X CYC:%4u",
            self.regs.pc as uint,
            disassembler.disassemble(),
            self.regs.a as uint,
            self.regs.x as uint,
            self.regs.y as uint,
            self.regs.flags as uint,
            self.regs.s as uint,
            self.cy as uint
        ));
    }
    #[cfg(ndebug)]
    fn trace(&mut self) {}

    // Memory access helpers
    /// Loads the byte at the program counter and increments the program counter.
    fn loadb_bump_pc(&mut self) -> u8 {
        let val = self.mem.loadb(self.regs.pc);
        self.regs.pc += 1;
        val
    }
    /// Loads two bytes (little-endian) at the program counter and bumps the program counter over
    /// them.
    fn loadw_bump_pc(&mut self) -> u16 {
        let val = self.mem.loadw(self.regs.pc);
        self.regs.pc += 2;
        val
    }

    // Stack helpers
    fn pushb(&mut self, val: u8) {
        self.mem.storeb(0x100 + self.regs.s as u16, val);
        self.regs.s -= 1;
    }
    fn pushw(&mut self, val: u16) {
        // FIXME: Is this correct? FCEU has two self.mem.storeb()s here. Might have different
        // semantics...
        self.mem.storew(0x100 + (self.regs.s - 1) as u16, val);
        self.regs.s -= 2;
    }
    fn popb(&mut self) -> u8 {
        let val = self.mem.loadb(0x100 + self.regs.s as u16 + 1);
        self.regs.s += 1;
        val
    }
    fn popw(&mut self) -> u16 {
        // FIXME: See comment in pushw().
        let val = self.mem.loadw(0x100 + self.regs.s as u16 + 1);
        self.regs.s += 2;
        val
    }

    // Flag helpers
    fn get_flag(&mut self, flag: u8) -> bool { (self.regs.flags & flag) != 0 }
    fn set_flag(&mut self, flag: u8, on: bool) {
        if on {
            self.regs.flags |= flag;
        } else {
            self.regs.flags &= !flag;
        }
    }
    fn set_zn(&mut self, val: u8) -> u8 {
        self.set_flag(ZERO_FLAG, val == 0);
        self.set_flag(NEGATIVE_FLAG, (val & 0x80) != 0);
        val
    }
    fn set_znv(&mut self, val: u8) -> u8 {
        let _ = self.set_zn(val);
        self.set_flag(OVERFLOW_FLAG, self.get_flag(CARRY_FLAG) ^ self.get_flag(NEGATIVE_FLAG));
        val
    }

    // Addressing modes
    fn zero_page(&mut self) -> MemoryAddressingMode {
        MemoryAddressingMode(self.loadb_bump_pc() as u16)
    }
    fn zero_page_x(&mut self) -> MemoryAddressingMode {
        MemoryAddressingMode(self.loadb_bump_pc() as u16 + self.regs.x as u16)
    }
    fn zero_page_y(&mut self) -> MemoryAddressingMode {
        MemoryAddressingMode(self.loadb_bump_pc() as u16 + self.regs.y as u16)
    }
    fn absolute(&mut self) -> MemoryAddressingMode {
        MemoryAddressingMode(self.loadw_bump_pc())
    }
    fn absolute_x(&mut self) -> MemoryAddressingMode {
        MemoryAddressingMode(self.loadw_bump_pc() + self.regs.x as u16)
    }
    fn absolute_y(&mut self) -> MemoryAddressingMode {
        MemoryAddressingMode(self.loadw_bump_pc() + self.regs.y as u16)
    }
    fn indexed_indirect_x(&mut self) -> MemoryAddressingMode {
        let addr = self.mem.loadb(self.loadb_bump_pc() as u16 + self.regs.x as u16) as u16;
        MemoryAddressingMode(addr)
    }
    fn indirect_indexed_y(&mut self) -> MemoryAddressingMode {
        let addr = self.mem.loadb(self.loadb_bump_pc() as u16) as u16 + self.regs.y as u16;
        MemoryAddressingMode(addr)
    }

    //
    // Instructions
    //

    // Loads
    fn lda<AM:AddressingMode<M>>(&mut self, am: AM) { self.regs.a = self.set_zn(am.load(self)) }
    fn ldx<AM:AddressingMode<M>>(&mut self, am: AM) { self.regs.x = self.set_zn(am.load(self)) }
    fn ldy<AM:AddressingMode<M>>(&mut self, am: AM) { self.regs.y = self.set_zn(am.load(self)) }

    // Stores
    fn sta<AM:AddressingMode<M>>(&mut self, am: AM) { am.store(self, self.regs.a) }
    fn stx<AM:AddressingMode<M>>(&mut self, am: AM) { am.store(self, self.regs.x) }
    fn sty<AM:AddressingMode<M>>(&mut self, am: AM) { am.store(self, self.regs.y) }

    // Arithmetic
    #[inline(always)]
    fn adc<AM:AddressingMode<M>>(&mut self, am: AM) {
        let val = am.load(self);
        let mut result = self.regs.a as u32 + val as u32;
        if self.get_flag(CARRY_FLAG) {
            result += 1;
        }

        self.set_flag(CARRY_FLAG, (result & 0x100) != 0);
        self.regs.a = self.set_znv(result as u8);
    }
    #[inline(always)]
    fn sbc<AM:AddressingMode<M>>(&mut self, am: AM) {
        let val = am.load(self);
        let mut result = self.regs.a as u32 - val as u32;
        if !self.get_flag(CARRY_FLAG) {
            result -= 1;
        }

        self.set_flag(CARRY_FLAG, (result & 0x100) == 0);
        self.regs.a = self.set_znv(result as u8);
    }

    // Comparisons
    fn cmp_base<AM:AddressingMode<M>>(&mut self, x: u8, am: AM) {
        let y = am.load(self);
        let mut result = x as u32 - y as u32;
        self.set_flag(CARRY_FLAG, (result & 0x100) == 0);
        self.regs.a = self.set_zn(result as u8);
    }
    fn cmp<AM:AddressingMode<M>>(&mut self, am: AM) { self.cmp_base(self.regs.a, am) }
    fn cpx<AM:AddressingMode<M>>(&mut self, am: AM) { self.cmp_base(self.regs.x, am) }
    fn cpy<AM:AddressingMode<M>>(&mut self, am: AM) { self.cmp_base(self.regs.y, am) }

    // Bitwise operations
    fn and<AM:AddressingMode<M>>(&mut self, am: AM) {
        self.regs.a = self.set_zn(am.load(self) & self.regs.a)
    }
    fn ora<AM:AddressingMode<M>>(&mut self, am: AM) {
        self.regs.a = self.set_zn(am.load(self) | self.regs.a)
    }
    fn eor<AM:AddressingMode<M>>(&mut self, am: AM) {
        self.regs.a = self.set_zn(am.load(self) ^ self.regs.a)
    }
    fn bit<AM:AddressingMode<M>>(&mut self, am: AM) {
        let val = am.load(self);
        self.set_flag(ZERO_FLAG, (val & self.regs.a) == 0);
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
        am.store(self, self.set_zn(result as u8))
    }
    fn shr_base<AM:AddressingMode<M>>(&mut self, msb: bool, am: AM) {
        let val = am.load(self);
        let new_carry = (val & 0x1) != 0;
        let mut result = val >> 1;
        if msb {
            result |= 0x80;
        }
        self.set_flag(CARRY_FLAG, new_carry);
        am.store(self, self.set_zn(result as u8))
    }
    fn rol<AM:AddressingMode<M>>(&mut self, am: AM) {
        self.shl_base(self.get_flag(CARRY_FLAG), am)
    }
    fn ror<AM:AddressingMode<M>>(&mut self, am: AM) {
        self.shr_base(self.get_flag(CARRY_FLAG), am)
    }
    fn asl<AM:AddressingMode<M>>(&mut self, am: AM) { self.shl_base(false, am) }
    fn lsr<AM:AddressingMode<M>>(&mut self, am: AM) { self.shr_base(false, am) }

    // Increments and decrements
    fn inc<AM:AddressingMode<M>>(&mut self, am: AM) {
        am.store(self, self.set_zn(am.load(self) + 1))
    }
    fn dec<AM:AddressingMode<M>>(&mut self, am: AM) {
        am.store(self, self.set_zn(am.load(self) - 1))
    }
    fn inx(&mut self) { self.regs.x = self.set_zn(self.regs.x + 1) }
    fn dex(&mut self) { self.regs.x = self.set_zn(self.regs.x - 1) }
    fn iny(&mut self) { self.regs.y = self.set_zn(self.regs.y + 1) }
    fn dey(&mut self) { self.regs.y = self.set_zn(self.regs.y - 1) }

    // Register moves
    fn tax(&mut self) { self.regs.x = self.set_zn(self.regs.a) }
    fn tay(&mut self) { self.regs.y = self.set_zn(self.regs.a) }
    fn txa(&mut self) { self.regs.a = self.set_zn(self.regs.x) }
    fn tya(&mut self) { self.regs.a = self.set_zn(self.regs.y) }
    fn txs(&mut self) { self.regs.s = self.regs.x }
    fn tsx(&mut self) { self.regs.x = self.regs.s }

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
        let disp = self.loadb_bump_pc() as i8;
        if cond {
            self.regs.pc = (self.regs.pc as i32 + disp as i32) as u16;
        }
    }
    fn bpl(&mut self) { self.bra_base(!self.get_flag(NEGATIVE_FLAG)) }
    fn bmi(&mut self) { self.bra_base(self.get_flag(NEGATIVE_FLAG))  }
    fn bvc(&mut self) { self.bra_base(!self.get_flag(OVERFLOW_FLAG)) }
    fn bvs(&mut self) { self.bra_base(self.get_flag(OVERFLOW_FLAG))  }
    fn bcc(&mut self) { self.bra_base(!self.get_flag(CARRY_FLAG))    }
    fn bcs(&mut self) { self.bra_base(self.get_flag(CARRY_FLAG))     }
    fn bne(&mut self) { self.bra_base(!self.get_flag(ZERO_FLAG))     }
    fn beq(&mut self) { self.bra_base(self.get_flag(ZERO_FLAG))      }

    // Jumps
    fn jmp(&mut self) { self.regs.pc = self.loadw_bump_pc() }
    fn jmpi(&mut self) {
        // Replicate the famous CPU bug...
        let pc_high = self.regs.pc & 0xff00;
        let lo = self.loadb_bump_pc();
        let hi = self.mem.loadb((self.regs.pc & 0x00ff) | pc_high);

        self.regs.pc = (hi as u16 << 8) | lo as u16;
    }

    // Procedure calls
    fn jsr(&mut self) {
        let addr = self.loadw_bump_pc();
        self.pushw(self.regs.pc - 1);
        self.regs.pc = addr;
    }
    fn rts(&mut self) { self.regs.pc = self.popw() + 1 }
    fn brk(&mut self) {
        self.pushw(self.regs.pc + 1);
        self.pushb(self.regs.flags);    // FIXME: FCEU sets BREAK_FLAG and U_FLAG here, why?
        self.set_flag(IRQ_FLAG, true);
        self.regs.pc = self.mem.loadw(BRK_VECTOR);
    }
    fn rti(&mut self) {
        self.regs.flags = self.popb();
        self.regs.pc = self.popw(); // NB: no + 1
    }

    // Stack operations
    fn pha(&mut self) { self.pushb(self.regs.a) }
    fn pla(&mut self) { self.regs.a = self.popb() }
    fn php(&mut self) { self.pushb(self.regs.flags) }
    fn plp(&mut self) { self.regs.flags = self.popb() }

    // No operation
    fn nop(&mut self) {}

    // The main fetch-and-decode routine
    fn step(&mut self) {
        self.trace();

        let op = self.loadb_bump_pc();
        decode_op!(
            op: op,
            this: self,
            modes: [
                ImmediateAddressingMode,
                AccumulatorAddressingMode,
                self.zero_page(),
                self.zero_page_x(),
                self.zero_page_y(),
                self.absolute(),
                self.absolute_x(),
                self.absolute_y(),
                self.indexed_indirect_x(),
                self.indirect_indexed_y()
            ]
        );

        self.cy += CYCLE_TABLE[op] as Cycles;
    }

    /// External interfaces
    fn reset(&mut self) {
        self.regs.pc = self.mem.loadw(RESET_VECTOR);
    }

    /// The constructor.
    static fn new(mem: M) -> Cpu<M> {
        Cpu {
            cy: 0,
            regs: Regs::new(),
            mem: mem
        }
    }
}

