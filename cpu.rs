//
// sprocketnes/cpu.rs
//
// Copyright (c) 2012 Mozilla Foundation
// Author: Patrick Walton
//

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

/// A basic memory interface used for testing.
pub struct SimpleMem {
    data: [u8 * 65536]  // FIXME: Stub.
}

impl SimpleMem : Mem {
    fn loadb(&mut self, addr: u16) -> u8     { self.data[addr] }
    fn storeb(&mut self, addr: u16, val: u8) { self.data[addr] = val }
    fn loadw(&mut self, addr: u16) -> u16 {
        // FIXME: On x86 use unsafe code to do an unaligned read.
        self.data[addr] as u16 | (self.data[addr + 1] as u16 << 8)
    }
    fn storew(&mut self, addr: u16, val: u16) {
        // FIXME: On x86 use unsafe code to do an unaligned store.
        self.data[addr] = val as u8;
        self.data[addr+1] = (val >> 8) as u8;
    }
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
    static fn new() -> Regs { Regs { a: 0, x: 0, y: 0, s: 0, flags: 0, pc: 0 } }
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
// Main CPU implementation
//

type Cycles = u64;

/// The main CPU structure definition.
pub struct Cpu<M> {
    cy: Cycles,
    regs: Regs,
    debug: CpuDebug,
    mem: M,
}

// Debugging
#[cfg(debug)]
pub struct CpuDebug {
    mnem: Option<&static/str>,
    cy_snapshot: Cycles,
    regs_snapshot: Regs,
}
#[cfg(ndebug)]
pub struct CpuDebug;

// FIXME: This should not need to be public! Sigh. Resolve bug.
pub impl CpuDebug {
    #[cfg(debug)]
    static fn new() -> CpuDebug {
        CpuDebug {
            mnem: None,
            cy_snapshot: 0,
            regs_snapshot: Regs::new()
        }
    }
    #[cfg(debug)]
    fn snapshot(&mut self, regs: &mut Regs) {
        self.regs_snapshot = *regs;
    }
    #[cfg(debug)]
    fn print(&mut self) {
        io::println(fmt!(
            "%04x %s A:%02x X:%02x Y:%02x P:%02x SP:%02x CYC:%4u",
            self.regs_snapshot.pc as uint,
            match self.mnem { None => "???", Some(m) => m },
            self.regs_snapshot.a as uint,
            self.regs_snapshot.x as uint,
            self.regs_snapshot.y as uint,
            self.regs_snapshot.flags as uint,
            self.regs_snapshot.s as uint,
            self.cy_snapshot as uint
        ));
    }

    #[cfg(ndebug)]
    static fn new() -> CpuDebug { CpuDebug }
    #[cfg(ndebug)]
    fn snapshot(&mut self, _: &mut Regs) {}
    #[cfg(ndebug)]
    fn print(&mut self) {}
}

// FIXME: This should not need to be public! Sigh. Resolve bug.
pub impl<M:Mem> Cpu<M> {
    // Debugging
    #[cfg(debug)]
    fn mnem(&mut self, mnemonic: &static/str) { self.debug.mnem = Some(mnemonic) }
    #[cfg(ndebug)]
    fn mnem(&mut self, _: &static/str) {}

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
    fn jmp(&mut self) { self.mnem("jmp"); self.regs.pc = self.loadw_bump_pc() }
    fn jmpi(&mut self) {
        self.mnem("jmp");

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

    // The main fetch-and-decode routine
    fn step(&mut self) {
        self.debug.snapshot(&mut self.regs);

        // We try to keep this in the same order as the implementations above.
        // TODO: Use arm macros to fix some of this duplication.
        let op = self.loadb_bump_pc();
        match op {
            // Loads
            0xa1 => self.lda(self.indexed_indirect_x()),
            0xa5 => self.lda(self.zero_page()),
            0xa9 => self.lda(ImmediateAddressingMode),
            0xad => self.lda(self.absolute()),
            0xb1 => self.lda(self.indirect_indexed_y()),
            0xb5 => self.lda(self.zero_page_x()),
            0xb9 => self.lda(self.absolute_y()),
            0xbd => self.lda(self.absolute_x()),

            0xa2 => self.ldx(ImmediateAddressingMode),
            0xa6 => self.ldx(self.zero_page()),
            0xae => self.ldx(self.zero_page_y()),
            0xb6 => self.ldx(self.absolute()),
            0xbe => self.ldx(self.absolute_y()),

            0xa0 => self.ldy(ImmediateAddressingMode),
            0xa4 => self.ldy(self.zero_page()),
            0xac => self.ldy(self.zero_page_x()),
            0xb4 => self.ldy(self.absolute()),
            0xbc => self.ldy(self.absolute_x()),

            // Stores
            0x85 => self.sta(self.zero_page()),
            0x95 => self.sta(self.zero_page_x()),
            0x8d => self.sta(self.absolute()),
            0x9d => self.sta(self.absolute_x()),
            0x99 => self.sta(self.absolute_y()),
            0x81 => self.sta(self.indexed_indirect_x()),
            0x91 => self.sta(self.indirect_indexed_y()),

            0x86 => self.stx(self.zero_page()),
            0x96 => self.stx(self.zero_page_y()),
            0x8e => self.stx(self.absolute()),

            0x84 => self.sty(self.zero_page()),
            0x94 => self.sty(self.zero_page_x()),
            0x8c => self.sty(self.absolute()),

            // Arithmetic
            0x69 => self.adc(ImmediateAddressingMode),
            0x65 => self.adc(self.zero_page()),
            0x75 => self.adc(self.zero_page_x()),
            0x6d => self.adc(self.absolute()),
            0x7d => self.adc(self.absolute_x()),
            0x79 => self.adc(self.absolute_y()),
            0x61 => self.adc(self.indexed_indirect_x()),
            0x71 => self.adc(self.indirect_indexed_y()),

            0xe9 => self.sbc(ImmediateAddressingMode),
            0xe5 => self.sbc(self.zero_page()),
            0xf5 => self.sbc(self.zero_page_x()),
            0xed => self.sbc(self.absolute()),
            0xfd => self.sbc(self.absolute_x()),
            0xf9 => self.sbc(self.absolute_y()),
            0xe1 => self.sbc(self.indexed_indirect_x()),
            0xf1 => self.sbc(self.indirect_indexed_y()),

            // Comparisons
            0xc9 => self.cmp(ImmediateAddressingMode),
            0xc5 => self.cmp(self.zero_page()),
            0xd5 => self.cmp(self.zero_page_x()),
            0xcd => self.cmp(self.absolute()),
            0xdd => self.cmp(self.absolute_x()),
            0xd9 => self.cmp(self.absolute_y()),
            0xc1 => self.cmp(self.indexed_indirect_x()),
            0xd1 => self.cmp(self.indirect_indexed_y()),

            0xe0 => self.cpx(ImmediateAddressingMode),
            0xe4 => self.cpx(self.zero_page()),
            0xec => self.cpx(self.absolute()),

            0xc0 => self.cpy(ImmediateAddressingMode),
            0xc4 => self.cpy(self.zero_page()),
            0xcc => self.cpy(self.absolute()),

            // Bitwise operations
            0x29 => self.and(ImmediateAddressingMode),
            0x25 => self.and(self.zero_page()),
            0x35 => self.and(self.zero_page_x()),
            0x2d => self.and(self.absolute()),
            0x3d => self.and(self.absolute_x()),
            0x39 => self.and(self.absolute_y()),
            0x21 => self.and(self.indexed_indirect_x()),
            0x31 => self.and(self.indirect_indexed_y()),

            0x09 => self.ora(ImmediateAddressingMode),
            0x05 => self.ora(self.zero_page()),
            0x15 => self.ora(self.zero_page_x()),
            0x0d => self.ora(self.absolute()),
            0x1d => self.ora(self.absolute_x()),
            0x19 => self.ora(self.absolute_y()),
            0x01 => self.ora(self.indexed_indirect_x()),
            0x11 => self.ora(self.indirect_indexed_y()),

            0x49 => self.eor(ImmediateAddressingMode),
            0x45 => self.eor(self.zero_page()),
            0x55 => self.eor(self.zero_page_x()),
            0x4d => self.eor(self.absolute()),
            0x5d => self.eor(self.absolute_x()),
            0x59 => self.eor(self.absolute_y()),
            0x41 => self.eor(self.indexed_indirect_x()),
            0x51 => self.eor(self.indirect_indexed_y()),

            0x24 => self.bit(self.zero_page()),
            0x2c => self.bit(self.absolute()),

            // Shifts and rotates
            0x2a => self.rol(AccumulatorAddressingMode),
            0x26 => self.rol(self.zero_page()),
            0x36 => self.rol(self.zero_page_x()),
            0x2e => self.rol(self.absolute()),
            0x3e => self.rol(self.absolute_x()),

            0x6a => self.ror(AccumulatorAddressingMode),
            0x66 => self.ror(self.zero_page()),
            0x76 => self.ror(self.zero_page_x()),
            0x6e => self.ror(self.absolute()),
            0x7e => self.ror(self.absolute_x()),

            0x0a => self.asl(AccumulatorAddressingMode),
            0x06 => self.asl(self.zero_page()),
            0x16 => self.asl(self.zero_page_x()),
            0x0e => self.asl(self.absolute()),
            0x1e => self.asl(self.absolute_x()),

            0x4a => self.lsr(AccumulatorAddressingMode),
            0x46 => self.lsr(self.zero_page()),
            0x56 => self.lsr(self.zero_page_x()),
            0x4e => self.lsr(self.absolute()),
            0x5e => self.lsr(self.absolute_x()),

            // Increments and decrements
            0xe6 => self.inc(self.zero_page()),
            0xf6 => self.inc(self.zero_page_x()),
            0xee => self.inc(self.absolute()),
            0xfe => self.inc(self.absolute_x()),

            0xc6 => self.dec(self.zero_page()),
            0xd6 => self.dec(self.zero_page_x()),
            0xce => self.dec(self.absolute()),
            0xde => self.dec(self.absolute_x()),

            0xe8 => self.inx(),
            0xca => self.dex(),
            0xc8 => self.iny(),
            0x88 => self.dey(),

            // Register moves
            0xaa => self.tax(),
            0xa8 => self.tay(),
            0x8a => self.txa(),
            0x98 => self.tya(),
            0x9a => self.txs(),
            0xba => self.tsx(),

            // Flag operations
            0x18 => self.clc(),
            0x38 => self.sec(),
            0x58 => self.cli(),
            0x78 => self.sei(),
            0xb8 => self.clv(),
            0xd8 => self.cld(),
            0xf8 => self.sed(),

            // Branches
            0x10 => self.bpl(),
            0x30 => self.bmi(),
            0x50 => self.bvc(),
            0x70 => self.bvs(),
            0x90 => self.bcc(),
            0xb0 => self.bcs(),
            0xd0 => self.bne(),
            0xf0 => self.beq(),

            // Jumps
            0x4c => self.jmp(),
            0x6c => self.jmpi(),

            // Procedure calls
            0x20 => self.jsr(),
            0x60 => self.rts(),
            0x00 => self.brk(),
            0x40 => self.rti(),

            // Stack operations
            0x48 => self.pha(),
            0x68 => self.pla(),
            0x08 => self.php(),
            0x28 => self.plp(),

            // No operation
            0xea => (),

            _ => fail ~"unimplemented or illegal instruction"
        }

        self.cy += CYCLE_TABLE[op] as Cycles;

        self.debug.print();
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
            debug: CpuDebug::new(),
            mem: mem
        }
    }
}

