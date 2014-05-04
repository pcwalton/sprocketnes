//
// sprocketnes/disasm.rs
//
// Author: Patrick Walton
//

use mem::Mem;

use libc::{uint8_t, uint16_t};

pub struct Disassembler<'a,M> {
    pub pc: uint16_t,
    pub mem: &'a mut M
}

impl<'a,M:Mem> Disassembler<'a,M> {
    //
    // Loads and byte-to-string conversion
    //

    fn loadb_bump_pc(&mut self) -> uint8_t {
        let val = (&mut *self.mem).loadb(self.pc);
        self.pc += 1;
        val
    }
    fn loadw_bump_pc(&mut self) -> uint16_t {
        let bottom = self.loadb_bump_pc() as uint16_t;
        let top = self.loadb_bump_pc() as uint16_t << 8;
        bottom | top
    }

    fn disb_bump_pc(&mut self) -> StrBuf {
        (format!("${:02X}", self.loadb_bump_pc() as uint)).to_strbuf()
    }
    fn disw_bump_pc(&mut self) -> StrBuf {
        (format!("${:04X}", self.loadw_bump_pc() as uint)).to_strbuf()
    }

    //
    // Mnemonics
    //

    // TODO: When we get method macros some of this ugly duplication can go away.

    // Loads
    fn lda(&mut self, am: StrBuf) -> StrBuf { (format!("LDA {}", am)).to_strbuf() }
    fn ldx(&mut self, am: StrBuf) -> StrBuf { (format!("LDX {}", am)).to_strbuf() }
    fn ldy(&mut self, am: StrBuf) -> StrBuf { (format!("LDY {}", am)).to_strbuf() }

    // Stores
    fn sta(&mut self, am: StrBuf) -> StrBuf { (format!("STA {}", am)).to_strbuf() }
    fn stx(&mut self, am: StrBuf) -> StrBuf { (format!("STX {}", am)).to_strbuf() }
    fn sty(&mut self, am: StrBuf) -> StrBuf { (format!("STY {}", am)).to_strbuf() }

    // Arithmetic
    fn adc(&mut self, am: StrBuf) -> StrBuf { (format!("ADC {}", am)).to_strbuf() }
    fn sbc(&mut self, am: StrBuf) -> StrBuf { (format!("SBC {}", am)).to_strbuf() }

    // Comparisons
    fn cmp(&mut self, am: StrBuf) -> StrBuf { (format!("CMP {}", am)).to_strbuf() }
    fn cpx(&mut self, am: StrBuf) -> StrBuf { (format!("CPX {}", am)).to_strbuf() }
    fn cpy(&mut self, am: StrBuf) -> StrBuf { (format!("CPY {}", am)).to_strbuf() }

    // Bitwise operations
    fn and(&mut self, am: StrBuf) -> StrBuf { (format!("AND {}", am)).to_strbuf() }
    fn ora(&mut self, am: StrBuf) -> StrBuf { (format!("ORA {}", am)).to_strbuf() }
    fn eor(&mut self, am: StrBuf) -> StrBuf { (format!("EOR {}", am)).to_strbuf() }
    fn bit(&mut self, am: StrBuf) -> StrBuf { (format!("BIT {}", am)).to_strbuf() }

    // Shifts and rotates
    fn rol(&mut self, am: StrBuf) -> StrBuf { (format!("ROL {}", am)).to_strbuf() }
    fn ror(&mut self, am: StrBuf) -> StrBuf { (format!("ROR {}", am)).to_strbuf() }
    fn asl(&mut self, am: StrBuf) -> StrBuf { (format!("ASL {}", am)).to_strbuf() }
    fn lsr(&mut self, am: StrBuf) -> StrBuf { (format!("LSR {}", am)).to_strbuf() }

    // Increments and decrements
    fn inc(&mut self, am: StrBuf) -> StrBuf { (format!("INC {}", am)).to_strbuf() }
    fn dec(&mut self, am: StrBuf) -> StrBuf { (format!("DEC {}", am)).to_strbuf() }
    fn inx(&mut self) -> StrBuf           { "INX".to_strbuf()       }
    fn dex(&mut self) -> StrBuf           { "DEX".to_strbuf()       }
    fn iny(&mut self) -> StrBuf           { "INY".to_strbuf()       }
    fn dey(&mut self) -> StrBuf           { "DEY".to_strbuf()       }

    // Register moves
    fn tax(&mut self) -> StrBuf           { "TAX".to_strbuf()       }
    fn tay(&mut self) -> StrBuf           { "TAY".to_strbuf()       }
    fn txa(&mut self) -> StrBuf           { "TXA".to_strbuf()       }
    fn tya(&mut self) -> StrBuf           { "TYA".to_strbuf()       }
    fn txs(&mut self) -> StrBuf           { "TXS".to_strbuf()       }
    fn tsx(&mut self) -> StrBuf           { "TSX".to_strbuf()       }

    // Flag operations
    fn clc(&mut self) -> StrBuf           { "CLC".to_strbuf()       }
    fn sec(&mut self) -> StrBuf           { "SEC".to_strbuf()       }
    fn cli(&mut self) -> StrBuf           { "CLI".to_strbuf()       }
    fn sei(&mut self) -> StrBuf           { "SEI".to_strbuf()       }
    fn clv(&mut self) -> StrBuf           { "CLV".to_strbuf()       }
    fn cld(&mut self) -> StrBuf           { "CLD".to_strbuf()       }
    fn sed(&mut self) -> StrBuf           { "SED".to_strbuf()       }

    // Branches
    // FIXME: Should disassemble the displacement!
    fn bpl(&mut self) -> StrBuf           { "BPL xx".to_strbuf()    }
    fn bmi(&mut self) -> StrBuf           { "BMI xx".to_strbuf()    }
    fn bvc(&mut self) -> StrBuf           { "BVC xx".to_strbuf()    }
    fn bvs(&mut self) -> StrBuf           { "BVS xx".to_strbuf()    }
    fn bcc(&mut self) -> StrBuf           { "BCC xx".to_strbuf()    }
    fn bcs(&mut self) -> StrBuf           { "BCS xx".to_strbuf()    }
    fn bne(&mut self) -> StrBuf           { "BNE xx".to_strbuf()    }
    fn beq(&mut self) -> StrBuf           { "BEQ xx".to_strbuf()    }

    // Jumps
    // FIXME: Should disassemble the address!
    fn jmp(&mut self) -> StrBuf           { "JMP xx".to_strbuf()    }
    fn jmpi(&mut self) -> StrBuf          { "JMP (xx)".to_strbuf()  }

    // Procedure calls
    // FIXME: Should disassemble the address!
    fn jsr(&mut self) -> StrBuf           { "JSR xx".to_strbuf()    }
    fn rts(&mut self) -> StrBuf           { "RTS".to_strbuf()       }
    fn brk(&mut self) -> StrBuf           { "BRK".to_strbuf()       }
    fn rti(&mut self) -> StrBuf           { "RTI".to_strbuf()       }

    // Stack operations
    fn pha(&mut self) -> StrBuf           { "PHA".to_strbuf()       }
    fn pla(&mut self) -> StrBuf           { "PLA".to_strbuf()       }
    fn php(&mut self) -> StrBuf           { "PHP".to_strbuf()       }
    fn plp(&mut self) -> StrBuf           { "PLP".to_strbuf()       }

    // No operation
    fn nop(&mut self) -> StrBuf           { "NOP".to_strbuf()       }

    // Addressing modes
    fn immediate(&mut self) -> StrBuf {
        (format!("{}{}", "#", self.disb_bump_pc())).to_strbuf()
    }
    fn accumulator(&mut self) -> StrBuf {
        StrBuf::new()
    }
    fn zero_page(&mut self) -> StrBuf {
        self.disb_bump_pc()
    }
    fn zero_page_x(&mut self) -> StrBuf {
        let mut buf = self.disb_bump_pc();
        buf.push_str(",X");
        buf
    }
    fn zero_page_y(&mut self) -> StrBuf {
        let mut buf = self.disb_bump_pc();
        buf.push_str(",Y");
        buf
    }
    fn absolute(&mut self) -> StrBuf           { self.disw_bump_pc()                       }
    fn absolute_x(&mut self) -> StrBuf {
        let mut buf = self.disw_bump_pc();
        buf.push_str(",X");
        buf
    }
    fn absolute_y(&mut self) -> StrBuf {
        let mut buf = self.disw_bump_pc();
        buf.push_str(",Y");
        buf
    }
    fn indexed_indirect_x(&mut self) -> StrBuf {
        (format!("({},X)", self.disb_bump_pc())).to_strbuf()
    }
    fn indirect_indexed_y(&mut self) -> StrBuf {
        (format!("({}),Y", self.disb_bump_pc())).to_strbuf()
    }

    // The main disassembly routine.
    #[inline(never)]
    pub fn disassemble(&mut self) -> StrBuf {
        let op = self.loadb_bump_pc();
        decode_op!(op, self)
    }
}

