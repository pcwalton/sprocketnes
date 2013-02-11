//
// sprocketnes/cpu.rs
//
// Author: Patrick Walton
//

use disasm::Disassembler;
use mem::{Mem, MemUtil};
use util::println;

use core::uint::range;

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

const NMI_VECTOR:   u16 = 0xfffa; 
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
        die!(~"can't store to immediate")
    }
}

struct MemoryAddressingMode(u16);
impl<M:Mem> MemoryAddressingMode : AddressingMode<M> {
    fn load(&self, cpu: &mut Cpu<M>) -> u8 { cpu.loadb(**self) }
    fn store(&self, cpu: &mut Cpu<M>, val: u8) { cpu.storeb(**self, val) }
}

//
// Opcode decoding
//
// This is implemented as a macro so that both the disassembler and the emulator can use it.
//

macro_rules! decode_op {
    (op: $op:expr, this: $this:expr) => {
        // We try to keep this in the same order as the implementations above.
        // TODO: Use arm macros to fix some of this duplication.
        match $op {
            // Loads
            0xa1 => $this.lda($this.indexed_indirect_x()),
            0xa5 => $this.lda($this.zero_page()),
            0xa9 => $this.lda($this.immediate()),
            0xad => $this.lda($this.absolute()),
            0xb1 => $this.lda($this.indirect_indexed_y()),
            0xb5 => $this.lda($this.zero_page_x()),
            0xb9 => $this.lda($this.absolute_y()),
            0xbd => $this.lda($this.absolute_x()),

            0xa2 => $this.ldx($this.immediate()),
            0xa6 => $this.ldx($this.zero_page()),
            0xb6 => $this.ldx($this.zero_page_y()),
            0xae => $this.ldx($this.absolute()),
            0xbe => $this.ldx($this.absolute_y()),

            0xa0 => $this.ldy($this.immediate()),
            0xa4 => $this.ldy($this.zero_page()),
            0xb4 => $this.ldy($this.zero_page_x()),
            0xac => $this.ldy($this.absolute()),
            0xbc => $this.ldy($this.absolute_x()),

            // Stores
            0x85 => $this.sta($this.zero_page()),
            0x95 => $this.sta($this.zero_page_x()),
            0x8d => $this.sta($this.absolute()),
            0x9d => $this.sta($this.absolute_x()),
            0x99 => $this.sta($this.absolute_y()),
            0x81 => $this.sta($this.indexed_indirect_x()),
            0x91 => $this.sta($this.indirect_indexed_y()),

            0x86 => $this.stx($this.zero_page()),
            0x96 => $this.stx($this.zero_page_y()),
            0x8e => $this.stx($this.absolute()),

            0x84 => $this.sty($this.zero_page()),
            0x94 => $this.sty($this.zero_page_x()),
            0x8c => $this.sty($this.absolute()),

            // Arithmetic
            0x69 => $this.adc($this.immediate()),
            0x65 => $this.adc($this.zero_page()),
            0x75 => $this.adc($this.zero_page_x()),
            0x6d => $this.adc($this.absolute()),
            0x7d => $this.adc($this.absolute_x()),
            0x79 => $this.adc($this.absolute_y()),
            0x61 => $this.adc($this.indexed_indirect_x()),
            0x71 => $this.adc($this.indirect_indexed_y()),

            0xe9 => $this.sbc($this.immediate()),
            0xe5 => $this.sbc($this.zero_page()),
            0xf5 => $this.sbc($this.zero_page_x()),
            0xed => $this.sbc($this.absolute()),
            0xfd => $this.sbc($this.absolute_x()),
            0xf9 => $this.sbc($this.absolute_y()),
            0xe1 => $this.sbc($this.indexed_indirect_x()),
            0xf1 => $this.sbc($this.indirect_indexed_y()),

            // Comparisons
            0xc9 => $this.cmp($this.immediate()),
            0xc5 => $this.cmp($this.zero_page()),
            0xd5 => $this.cmp($this.zero_page_x()),
            0xcd => $this.cmp($this.absolute()),
            0xdd => $this.cmp($this.absolute_x()),
            0xd9 => $this.cmp($this.absolute_y()),
            0xc1 => $this.cmp($this.indexed_indirect_x()),
            0xd1 => $this.cmp($this.indirect_indexed_y()),

            0xe0 => $this.cpx($this.immediate()),
            0xe4 => $this.cpx($this.zero_page()),
            0xec => $this.cpx($this.absolute()),

            0xc0 => $this.cpy($this.immediate()),
            0xc4 => $this.cpy($this.zero_page()),
            0xcc => $this.cpy($this.absolute()),

            // Bitwise operations
            0x29 => $this.and($this.immediate()),
            0x25 => $this.and($this.zero_page()),
            0x35 => $this.and($this.zero_page_x()),
            0x2d => $this.and($this.absolute()),
            0x3d => $this.and($this.absolute_x()),
            0x39 => $this.and($this.absolute_y()),
            0x21 => $this.and($this.indexed_indirect_x()),
            0x31 => $this.and($this.indirect_indexed_y()),

            0x09 => $this.ora($this.immediate()),
            0x05 => $this.ora($this.zero_page()),
            0x15 => $this.ora($this.zero_page_x()),
            0x0d => $this.ora($this.absolute()),
            0x1d => $this.ora($this.absolute_x()),
            0x19 => $this.ora($this.absolute_y()),
            0x01 => $this.ora($this.indexed_indirect_x()),
            0x11 => $this.ora($this.indirect_indexed_y()),

            0x49 => $this.eor($this.immediate()),
            0x45 => $this.eor($this.zero_page()),
            0x55 => $this.eor($this.zero_page_x()),
            0x4d => $this.eor($this.absolute()),
            0x5d => $this.eor($this.absolute_x()),
            0x59 => $this.eor($this.absolute_y()),
            0x41 => $this.eor($this.indexed_indirect_x()),
            0x51 => $this.eor($this.indirect_indexed_y()),

            0x24 => $this.bit($this.zero_page()),
            0x2c => $this.bit($this.absolute()),

            // Shifts and rotates
            0x2a => $this.rol($this.accumulator()),
            0x26 => $this.rol($this.zero_page()),
            0x36 => $this.rol($this.zero_page_x()),
            0x2e => $this.rol($this.absolute()),
            0x3e => $this.rol($this.absolute_x()),

            0x6a => $this.ror($this.accumulator()),
            0x66 => $this.ror($this.zero_page()),
            0x76 => $this.ror($this.zero_page_x()),
            0x6e => $this.ror($this.absolute()),
            0x7e => $this.ror($this.absolute_x()),

            0x0a => $this.asl($this.accumulator()),
            0x06 => $this.asl($this.zero_page()),
            0x16 => $this.asl($this.zero_page_x()),
            0x0e => $this.asl($this.absolute()),
            0x1e => $this.asl($this.absolute_x()),

            0x4a => $this.lsr($this.accumulator()),
            0x46 => $this.lsr($this.zero_page()),
            0x56 => $this.lsr($this.zero_page_x()),
            0x4e => $this.lsr($this.absolute()),
            0x5e => $this.lsr($this.absolute_x()),

            // Increments and decrements
            0xe6 => $this.inc($this.zero_page()),
            0xf6 => $this.inc($this.zero_page_x()),
            0xee => $this.inc($this.absolute()),
            0xfe => $this.inc($this.absolute_x()),

            0xc6 => $this.dec($this.zero_page()),
            0xd6 => $this.dec($this.zero_page_x()),
            0xce => $this.dec($this.absolute()),
            0xde => $this.dec($this.absolute_x()),

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

            _ => die!(~"unimplemented or illegal instruction")
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

// The CPU implements Mem so that it can handle writes to the DMA register.
impl<M:Mem> Mem for Cpu<M> {
    fn loadb(&mut self, addr: u16) -> u8 { self.mem.loadb(addr) }

    fn storeb(&mut self, addr: u16, val: u8) {
        // Handle OAM_DMA.
        if addr == 0x4014 {
            self.dma(val);
            return;
        }

        self.mem.storeb(addr, val)
    }
}

impl<M:Mem> Cpu<M> {
    // Debugging
    #[cfg(cpuspew)]
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
    #[cfg(ncpuspew)]
    fn trace(&mut self) {}

    // Performs DMA to the OAMDATA ($2004) register.
    fn dma(&mut self, hi_addr: u8) {
        for range(hi_addr as uint << 8, (hi_addr + 1) as uint << 8) |addr| {
            self.storeb(0x2004, self.loadb(addr as u16));

            // FIXME: The last address sometimes takes 1 cycle, sometimes 2 -- NESdev isn't very
            // clear on this.
            self.cy += 2;
        }
    }

    // Memory access helpers
    /// Loads the byte at the program counter and increments the program counter.
    fn loadb_bump_pc(&mut self) -> u8 {
        let val = self.loadb(self.regs.pc);
        self.regs.pc += 1;
        val
    }
    /// Loads two bytes (little-endian) at the program counter and bumps the program counter over
    /// them.
    fn loadw_bump_pc(&mut self) -> u16 {
        let val = self.loadw(self.regs.pc);
        self.regs.pc += 2;
        val
    }

    // Stack helpers
    fn pushb(&mut self, val: u8) {
        self.storeb(0x100 + self.regs.s as u16, val);
        self.regs.s -= 1;
    }
    fn pushw(&mut self, val: u16) {
        // FIXME: Is this correct? FCEU has two self.storeb()s here. Might have different
        // semantics...
        self.storew(0x100 + (self.regs.s - 1) as u16, val);
        self.regs.s -= 2;
    }
    fn popb(&mut self) -> u8 {
        let val = self.loadb(0x100 + self.regs.s as u16 + 1);
        self.regs.s += 1;
        val
    }
    fn popw(&mut self) -> u16 {
        // FIXME: See comment in pushw().
        let val = self.loadw(0x100 + self.regs.s as u16 + 1);
        self.regs.s += 2;
        val
    }

    // Flag helpers
    fn get_flag(&self, flag: u8) -> bool { (self.regs.flags & flag) != 0 }
    fn set_flag(&mut self, flag: u8, on: bool) {
        if on {
            self.regs.flags |= flag;
        } else {
            self.regs.flags &= !flag;
        }
    }
    fn set_flags(&mut self, val: u8) {
        // Flags get munged in a strange way relating to the unused bit 5 on the NES.
        self.regs.flags = (val | 0x30) - 0x10;
    }
    fn set_zn(&mut self, val: u8) -> u8 {
        self.set_flag(ZERO_FLAG, val == 0);
        self.set_flag(NEGATIVE_FLAG, (val & 0x80) != 0);
        val
    }

    // Addressing modes
    fn immediate(&mut self) -> ImmediateAddressingMode { ImmediateAddressingMode }
    fn accumulator(&mut self) -> AccumulatorAddressingMode { AccumulatorAddressingMode }
    fn zero_page(&mut self) -> MemoryAddressingMode {
        MemoryAddressingMode(self.loadb_bump_pc() as u16)
    }
    fn zero_page_x(&mut self) -> MemoryAddressingMode {
        MemoryAddressingMode((self.loadb_bump_pc() + self.regs.x) as u16)
    }
    fn zero_page_y(&mut self) -> MemoryAddressingMode {
        MemoryAddressingMode((self.loadb_bump_pc() + self.regs.y) as u16)
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
        let addr = self.loadw_zp(self.loadb_bump_pc() + self.regs.x);
        MemoryAddressingMode(addr)
    }
    fn indirect_indexed_y(&mut self) -> MemoryAddressingMode {
        let addr = self.loadw_zp(self.loadb_bump_pc()) + self.regs.y as u16;
        MemoryAddressingMode(addr)
    }

    //
    // Instructions
    //

    // Loads
    fn lda<AM:AddressingMode<M>>(&mut self, am: AM) {
        self.regs.a = self.set_zn(am.load(&mut *self))
    }
    fn ldx<AM:AddressingMode<M>>(&mut self, am: AM) {
        self.regs.x = self.set_zn(am.load(&mut *self))
    }
    fn ldy<AM:AddressingMode<M>>(&mut self, am: AM) {
        self.regs.y = self.set_zn(am.load(&mut *self))
    }

    // Stores
    fn sta<AM:AddressingMode<M>>(&mut self, am: AM) { am.store(&mut *self, self.regs.a) }
    fn stx<AM:AddressingMode<M>>(&mut self, am: AM) { am.store(&mut *self, self.regs.x) }
    fn sty<AM:AddressingMode<M>>(&mut self, am: AM) { am.store(&mut *self, self.regs.y) }

    // Arithmetic
    #[inline(always)]
    fn adc<AM:AddressingMode<M>>(&mut self, am: AM) {
        let val = am.load(self);
        let mut result = self.regs.a as u32 + val as u32;
        if self.get_flag(CARRY_FLAG) {
            result += 1;
        }

        self.set_flag(CARRY_FLAG, (result & 0x100) != 0);

        let result = result as u8;
        self.set_flag(OVERFLOW_FLAG,
                      (self.regs.a ^ val) & 0x80 == 0 && (self.regs.a ^ result) & 0x80 == 0x80);
        self.regs.a = self.set_zn(result);
    }
    #[inline(always)]
    fn sbc<AM:AddressingMode<M>>(&mut self, am: AM) {
        let val = am.load(self);
        let mut result = self.regs.a as u32 - val as u32;
        if !self.get_flag(CARRY_FLAG) {
            result -= 1;
        }

        self.set_flag(CARRY_FLAG, (result & 0x100) == 0);

        let result = result as u8;
        self.set_flag(OVERFLOW_FLAG,
                      (self.regs.a ^ result) & 0x80 != 0 && (self.regs.a ^ val) & 0x80 == 0x80);
        self.regs.a = self.set_zn(result);
    }

    // Comparisons
    fn cmp_base<AM:AddressingMode<M>>(&mut self, x: u8, am: AM) {
        let y = am.load(&mut *self);
        let mut result = x as u32 - y as u32;
        (&mut *self).set_flag(CARRY_FLAG, (result & 0x100) == 0);
        let _ = (&mut *self).set_zn(result as u8);
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
        let val = am.load(&mut *self);
        self.set_flag(ZERO_FLAG, (val & self.regs.a) == 0);
        self.set_flag(NEGATIVE_FLAG, (val & 0x80) != 0);
        self.set_flag(OVERFLOW_FLAG, (val & 0x40) != 0);
    }

    // Shifts and rotates
    fn shl_base<AM:AddressingMode<M>>(&mut self, lsb: bool, am: AM) {
        let val = am.load(&mut *self);
        let new_carry = (val & 0x80) != 0;
        let mut result = val << 1;
        if lsb {
            result |= 1;
        }
        self.set_flag(CARRY_FLAG, new_carry);
        am.store(&mut *self, self.set_zn(result as u8))
    }
    fn shr_base<AM:AddressingMode<M>>(&mut self, msb: bool, am: AM) {
        let val = am.load(&mut *self);
        let new_carry = (val & 0x1) != 0;
        let mut result = val >> 1;
        if msb {
            result |= 0x80;
        }
        self.set_flag(CARRY_FLAG, new_carry);
        am.store(&mut *self, self.set_zn(result as u8))
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
        am.store(&mut *self, self.set_zn(am.load(self) + 1))
    }
    fn dec<AM:AddressingMode<M>>(&mut self, am: AM) {
        am.store(&mut *self, self.set_zn(am.load(self) - 1))
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
    fn tsx(&mut self) { self.regs.x = self.set_zn(self.regs.s) }

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
        let addr = self.loadw_bump_pc();

        // Replicate the famous CPU bug...
        let lo = self.loadb(addr);
        let hi = self.loadb((addr & 0xff00) | ((addr + 1) & 0x00ff));

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
        self.regs.pc = self.loadw(BRK_VECTOR);
    }
    fn rti(&mut self) {
        self.set_flags(self.popb());
        self.regs.pc = self.popw(); // NB: no + 1
    }

    // Stack operations
    fn pha(&mut self) { self.pushb(self.regs.a) }
    fn pla(&mut self) { self.regs.a = self.set_zn(self.popb()) }
    fn php(&mut self) { self.pushb(self.regs.flags | BREAK_FLAG) }
    fn plp(&mut self) { self.set_flags(self.popb()) }

    // No operation
    fn nop(&mut self) {}

    // The main fetch-and-decode routine
    fn step(&mut self) {
        self.trace();

        let op = self.loadb_bump_pc();
        decode_op!(op: op, this: self);

        self.cy += CYCLE_TABLE[op] as Cycles;
    }

    /// External interfaces
    fn reset(&mut self) {
        self.regs.pc = self.loadw(RESET_VECTOR);
    }
    fn nmi(&mut self) {
        self.pushw(self.regs.pc);
        self.pushb(self.regs.flags);
        self.regs.pc = self.loadw(NMI_VECTOR);
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

