//
// Author: Patrick Walton
//

use mem::Mem;

pub struct Disassembler<'a, M: Mem + 'a> {
    pub pc: u16,
    pub mem: &'a mut M,
}

impl<'a, M: Mem> Disassembler<'a, M> {
    //
    // Loads and byte-to-string conversion
    //

    fn loadb_bump_pc(&mut self) -> u8 {
        let val = (&mut *self.mem).loadb(self.pc);
        self.pc += 1;
        val
    }
    fn loadw_bump_pc(&mut self) -> u16 {
        let bottom = self.loadb_bump_pc() as u16;
        let top = (self.loadb_bump_pc() as u16) << 8;
        bottom | top
    }

    fn disb_bump_pc(&mut self) -> String {
        format!("${:02X}", self.loadb_bump_pc() as usize)
    }
    fn disw_bump_pc(&mut self) -> String {
        format!("${:04X}", self.loadw_bump_pc() as usize)
    }

    //
    // Mnemonics
    //

    // TODO: When we get method macros some of this ugly duplication can go away.

    // Loads
    fn lda(&mut self, am: String) -> String {
        (format!("LDA {}", am)).to_string()
    }
    fn ldx(&mut self, am: String) -> String {
        (format!("LDX {}", am)).to_string()
    }
    fn ldy(&mut self, am: String) -> String {
        (format!("LDY {}", am)).to_string()
    }

    // Stores
    fn sta(&mut self, am: String) -> String {
        (format!("STA {}", am)).to_string()
    }
    fn stx(&mut self, am: String) -> String {
        (format!("STX {}", am)).to_string()
    }
    fn sty(&mut self, am: String) -> String {
        (format!("STY {}", am)).to_string()
    }

    // Arithmetic
    fn adc(&mut self, am: String) -> String {
        (format!("ADC {}", am)).to_string()
    }
    fn sbc(&mut self, am: String) -> String {
        (format!("SBC {}", am)).to_string()
    }

    // Comparisons
    fn cmp(&mut self, am: String) -> String {
        (format!("CMP {}", am)).to_string()
    }
    fn cpx(&mut self, am: String) -> String {
        (format!("CPX {}", am)).to_string()
    }
    fn cpy(&mut self, am: String) -> String {
        (format!("CPY {}", am)).to_string()
    }

    // Bitwise operations
    fn and(&mut self, am: String) -> String {
        (format!("AND {}", am)).to_string()
    }
    fn ora(&mut self, am: String) -> String {
        (format!("ORA {}", am)).to_string()
    }
    fn eor(&mut self, am: String) -> String {
        (format!("EOR {}", am)).to_string()
    }
    fn bit(&mut self, am: String) -> String {
        (format!("BIT {}", am)).to_string()
    }

    // Shifts and rotates
    fn rol(&mut self, am: String) -> String {
        (format!("ROL {}", am)).to_string()
    }
    fn ror(&mut self, am: String) -> String {
        (format!("ROR {}", am)).to_string()
    }
    fn asl(&mut self, am: String) -> String {
        (format!("ASL {}", am)).to_string()
    }
    fn lsr(&mut self, am: String) -> String {
        (format!("LSR {}", am)).to_string()
    }

    // Increments and decrements
    fn inc(&mut self, am: String) -> String {
        (format!("INC {}", am)).to_string()
    }
    fn dec(&mut self, am: String) -> String {
        (format!("DEC {}", am)).to_string()
    }
    fn inx(&mut self) -> String {
        "INX".to_string()
    }
    fn dex(&mut self) -> String {
        "DEX".to_string()
    }
    fn iny(&mut self) -> String {
        "INY".to_string()
    }
    fn dey(&mut self) -> String {
        "DEY".to_string()
    }

    // Register moves
    fn tax(&mut self) -> String {
        "TAX".to_string()
    }
    fn tay(&mut self) -> String {
        "TAY".to_string()
    }
    fn txa(&mut self) -> String {
        "TXA".to_string()
    }
    fn tya(&mut self) -> String {
        "TYA".to_string()
    }
    fn txs(&mut self) -> String {
        "TXS".to_string()
    }
    fn tsx(&mut self) -> String {
        "TSX".to_string()
    }

    // Flag operations
    fn clc(&mut self) -> String {
        "CLC".to_string()
    }
    fn sec(&mut self) -> String {
        "SEC".to_string()
    }
    fn cli(&mut self) -> String {
        "CLI".to_string()
    }
    fn sei(&mut self) -> String {
        "SEI".to_string()
    }
    fn clv(&mut self) -> String {
        "CLV".to_string()
    }
    fn cld(&mut self) -> String {
        "CLD".to_string()
    }
    fn sed(&mut self) -> String {
        "SED".to_string()
    }

    // Branches
    // FIXME: Should disassemble the displacement!
    fn bpl(&mut self) -> String {
        "BPL xx".to_string()
    }
    fn bmi(&mut self) -> String {
        "BMI xx".to_string()
    }
    fn bvc(&mut self) -> String {
        "BVC xx".to_string()
    }
    fn bvs(&mut self) -> String {
        "BVS xx".to_string()
    }
    fn bcc(&mut self) -> String {
        "BCC xx".to_string()
    }
    fn bcs(&mut self) -> String {
        "BCS xx".to_string()
    }
    fn bne(&mut self) -> String {
        "BNE xx".to_string()
    }
    fn beq(&mut self) -> String {
        "BEQ xx".to_string()
    }

    // Jumps
    // FIXME: Should disassemble the address!
    fn jmp(&mut self) -> String {
        "JMP xx".to_string()
    }
    fn jmpi(&mut self) -> String {
        "JMP (xx)".to_string()
    }

    // Procedure calls
    // FIXME: Should disassemble the address!
    fn jsr(&mut self) -> String {
        "JSR xx".to_string()
    }
    fn rts(&mut self) -> String {
        "RTS".to_string()
    }
    fn brk(&mut self) -> String {
        "BRK".to_string()
    }
    fn rti(&mut self) -> String {
        "RTI".to_string()
    }

    // Stack operations
    fn pha(&mut self) -> String {
        "PHA".to_string()
    }
    fn pla(&mut self) -> String {
        "PLA".to_string()
    }
    fn php(&mut self) -> String {
        "PHP".to_string()
    }
    fn plp(&mut self) -> String {
        "PLP".to_string()
    }

    // No operation
    fn nop(&mut self) -> String {
        "NOP".to_string()
    }

    // Addressing modes
    fn immediate(&mut self) -> String {
        (format!("{}{}", "#", self.disb_bump_pc())).to_string()
    }
    fn accumulator(&mut self) -> String {
        String::new()
    }
    fn zero_page(&mut self) -> String {
        self.disb_bump_pc()
    }
    fn zero_page_x(&mut self) -> String {
        let mut buf = self.disb_bump_pc();
        buf.push_str(",X");
        buf
    }
    fn zero_page_y(&mut self) -> String {
        let mut buf = self.disb_bump_pc();
        buf.push_str(",Y");
        buf
    }
    fn absolute(&mut self) -> String {
        self.disw_bump_pc()
    }
    fn absolute_x(&mut self) -> String {
        let mut buf = self.disw_bump_pc();
        buf.push_str(",X");
        buf
    }
    fn absolute_y(&mut self) -> String {
        let mut buf = self.disw_bump_pc();
        buf.push_str(",Y");
        buf
    }
    fn indexed_indirect_x(&mut self) -> String {
        (format!("({},X)", self.disb_bump_pc())).to_string()
    }
    fn indirect_indexed_y(&mut self) -> String {
        (format!("({}),Y", self.disb_bump_pc())).to_string()
    }

    // The main disassembly routine.
    #[inline(never)]
    pub fn disassemble(&mut self) -> String {
        let op = self.loadb_bump_pc();
        decode_op!(op, self)
    }
}
