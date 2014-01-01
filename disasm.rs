//
// sprocketnes/disasm.rs
//
// Author: Patrick Walton
//

use mem::Mem;

pub struct Disassembler<'a,M> {
    pc: u16,
    mem: &'a mut M
}

impl<'a,M:Mem> Disassembler<'a,M> {
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
        let top = self.loadb_bump_pc() as u16 << 8;
        bottom | top
    }

    fn disb_bump_pc(&mut self) -> ~str { format!("${:02X}", self.loadb_bump_pc() as uint) }
    fn disw_bump_pc(&mut self) -> ~str { format!("${:04X}", self.loadw_bump_pc() as uint) }

    //
    // Mnemonics
    //

    // TODO: When we get method macros some of this ugly duplication can go away.

    // Loads
    fn lda(&mut self, am: ~str) -> ~str { format!("LDA {}", am) }
    fn ldx(&mut self, am: ~str) -> ~str { format!("LDX {}", am) }
    fn ldy(&mut self, am: ~str) -> ~str { format!("LDY {}", am) }

    // Stores
    fn sta(&mut self, am: ~str) -> ~str { format!("STA {}", am) }
    fn stx(&mut self, am: ~str) -> ~str { format!("STX {}", am) }
    fn sty(&mut self, am: ~str) -> ~str { format!("STY {}", am) }

    // Arithmetic
    fn adc(&mut self, am: ~str) -> ~str { format!("ADC {}", am) }
    fn sbc(&mut self, am: ~str) -> ~str { format!("SBC {}", am) }

    // Comparisons
    fn cmp(&mut self, am: ~str) -> ~str { format!("CMP {}", am) }
    fn cpx(&mut self, am: ~str) -> ~str { format!("CPX {}", am) }
    fn cpy(&mut self, am: ~str) -> ~str { format!("CPY {}", am) }

    // Bitwise operations
    fn and(&mut self, am: ~str) -> ~str { format!("AND {}", am) }
    fn ora(&mut self, am: ~str) -> ~str { format!("ORA {}", am) }
    fn eor(&mut self, am: ~str) -> ~str { format!("EOR {}", am) }
    fn bit(&mut self, am: ~str) -> ~str { format!("BIT {}", am) }

    // Shifts and rotates
    fn rol(&mut self, am: ~str) -> ~str { format!("ROL {}", am) }
    fn ror(&mut self, am: ~str) -> ~str { format!("ROR {}", am) }
    fn asl(&mut self, am: ~str) -> ~str { format!("ASL {}", am) }
    fn lsr(&mut self, am: ~str) -> ~str { format!("LSR {}", am) }

    // Increments and decrements
    fn inc(&mut self, am: ~str) -> ~str { format!("INC {}", am) }
    fn dec(&mut self, am: ~str) -> ~str { format!("DEC {}", am) }
    fn inx(&mut self) -> ~str           { "INX".to_str()       }
    fn dex(&mut self) -> ~str           { "DEX".to_str()       }
    fn iny(&mut self) -> ~str           { "INY".to_str()       }
    fn dey(&mut self) -> ~str           { "DEY".to_str()       }

    // Register moves
    fn tax(&mut self) -> ~str           { "TAX".to_str()       }
    fn tay(&mut self) -> ~str           { "TAY".to_str()       }
    fn txa(&mut self) -> ~str           { "TXA".to_str()       }
    fn tya(&mut self) -> ~str           { "TYA".to_str()       }
    fn txs(&mut self) -> ~str           { "TXS".to_str()       }
    fn tsx(&mut self) -> ~str           { "TSX".to_str()       }

    // Flag operations
    fn clc(&mut self) -> ~str           { "CLC".to_str()       }
    fn sec(&mut self) -> ~str           { "SEC".to_str()       }
    fn cli(&mut self) -> ~str           { "CLI".to_str()       }
    fn sei(&mut self) -> ~str           { "SEI".to_str()       }
    fn clv(&mut self) -> ~str           { "CLV".to_str()       }
    fn cld(&mut self) -> ~str           { "CLD".to_str()       }
    fn sed(&mut self) -> ~str           { "SED".to_str()       }

    // Branches
    // FIXME: Should disassemble the displacement!
    fn bpl(&mut self) -> ~str           { "BPL xx".to_str()    }
    fn bmi(&mut self) -> ~str           { "BMI xx".to_str()    }
    fn bvc(&mut self) -> ~str           { "BVC xx".to_str()    }
    fn bvs(&mut self) -> ~str           { "BVS xx".to_str()    }
    fn bcc(&mut self) -> ~str           { "BCC xx".to_str()    }
    fn bcs(&mut self) -> ~str           { "BCS xx".to_str()    }
    fn bne(&mut self) -> ~str           { "BNE xx".to_str()    }
    fn beq(&mut self) -> ~str           { "BEQ xx".to_str()    }

    // Jumps
    // FIXME: Should disassemble the address!
    fn jmp(&mut self) -> ~str           { "JMP xx".to_str()    }
    fn jmpi(&mut self) -> ~str          { "JMP (xx)".to_str()  }

    // Procedure calls
    // FIXME: Should disassemble the address!
    fn jsr(&mut self) -> ~str           { "JSR xx".to_str()    }
    fn rts(&mut self) -> ~str           { "RTS".to_str()       }
    fn brk(&mut self) -> ~str           { "BRK".to_str()       }
    fn rti(&mut self) -> ~str           { "RTI".to_str()       }

    // Stack operations
    fn pha(&mut self) -> ~str           { "PHA".to_str()       }
    fn pla(&mut self) -> ~str           { "PLA".to_str()       }
    fn php(&mut self) -> ~str           { "PHP".to_str()       }
    fn plp(&mut self) -> ~str           { "PLP".to_str()       }

    // No operation
    fn nop(&mut self) -> ~str           { "NOP".to_str()       }

    // Addressing modes
    fn immediate(&mut self) -> ~str          { format!("{}{}", "#", self.disb_bump_pc()) }
    fn accumulator(&mut self) -> ~str        { "".to_str()                               }
    fn zero_page(&mut self) -> ~str          { self.disb_bump_pc()                       }
    fn zero_page_x(&mut self) -> ~str        { self.disb_bump_pc() + ",X"                }
    fn zero_page_y(&mut self) -> ~str        { self.disb_bump_pc() + ",Y"                }
    fn absolute(&mut self) -> ~str           { self.disw_bump_pc()                       }
    fn absolute_x(&mut self) -> ~str         { self.disw_bump_pc() + ",X"                }
    fn absolute_y(&mut self) -> ~str         { self.disw_bump_pc() + ",Y"                }
    fn indexed_indirect_x(&mut self) -> ~str { format!("({},X)", self.disb_bump_pc())    }
    fn indirect_indexed_y(&mut self) -> ~str { format!("({}),Y", self.disb_bump_pc())    }

    // The main disassembly routine.
    #[inline(never)]
    pub fn disassemble(&mut self) -> ~str {
        let op = self.loadb_bump_pc();
        decode_op!(op, self)
    }
}

