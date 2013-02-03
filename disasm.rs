//
// sprocketnes/disasm.rs
//
// Copyright (c) 2012 Mozilla Foundation
// Author: Patrick Walton
//

use mem::Mem;

use core::uint;

pub struct Disassembler<M> {
    pc: u16,
    mem: &mut M
}

// FIXME: This should not need to be public! Sigh. Resolve bug.
pub impl<M:Mem> Disassembler<M> {
    //
    // Loads and byte-to-string conversion
    //

    fn loadb_bump_pc(&mut self) -> u8 {
        let val = self.mem.loadb(self.pc);
        self.pc += 1;
        val
    }
    fn loadw_bump_pc(&mut self) -> u16 {
        self.loadb_bump_pc() as u16 | (self.loadb_bump_pc() as u16 << 8)
    }

    fn disb_bump_pc(&mut self) -> ~str { fmt!("$%02X", self.loadb_bump_pc() as uint) }
    fn disw_bump_pc(&mut self) -> ~str { fmt!("$%04X", self.loadw_bump_pc() as uint) }

    //
    // Mnemonics
    //

    // TODO: When we get method macros some of this ugly duplication can go away.

    // Loads
    fn lda(&mut self, am: ~str) -> ~str { ~"LDA " + am }
    fn ldx(&mut self, am: ~str) -> ~str { ~"LDX " + am }
    fn ldy(&mut self, am: ~str) -> ~str { ~"LDY " + am }

    // Stores
    fn sta(&mut self, am: ~str) -> ~str { ~"STA " + am }
    fn stx(&mut self, am: ~str) -> ~str { ~"STX " + am }
    fn sty(&mut self, am: ~str) -> ~str { ~"STY " + am }

    // Arithmetic
    fn adc(&mut self, am: ~str) -> ~str { ~"ADC " + am }
    fn sbc(&mut self, am: ~str) -> ~str { ~"SBC " + am }

    // Comparisons
    fn cmp(&mut self, am: ~str) -> ~str { ~"CMP " + am }
    fn cpx(&mut self, am: ~str) -> ~str { ~"CPX " + am }
    fn cpy(&mut self, am: ~str) -> ~str { ~"CPY " + am }

    // Bitwise operations
    fn and(&mut self, am: ~str) -> ~str { ~"AND " + am }
    fn ora(&mut self, am: ~str) -> ~str { ~"ORA " + am }
    fn eor(&mut self, am: ~str) -> ~str { ~"EOR " + am }
    fn bit(&mut self, am: ~str) -> ~str { ~"BIT " + am }

    // Shifts and rotates
    fn rol(&mut self, am: ~str) -> ~str { ~"ROL " + am }
    fn ror(&mut self, am: ~str) -> ~str { ~"ROR " + am }
    fn asl(&mut self, am: ~str) -> ~str { ~"ASL " + am }
    fn lsr(&mut self, am: ~str) -> ~str { ~"LSR " + am }

    // Increments and decrements
    fn inc(&mut self, am: ~str) -> ~str { ~"INC " + am }
    fn dec(&mut self, am: ~str) -> ~str { ~"DEC " + am }
    fn inx(&mut self) -> ~str           { ~"INX"       }
    fn dex(&mut self) -> ~str           { ~"DEX"       }
    fn iny(&mut self) -> ~str           { ~"INY"       }
    fn dey(&mut self) -> ~str           { ~"DEY"       }

    // Register moves
    fn tax(&mut self) -> ~str           { ~"TAX"       }
    fn tay(&mut self) -> ~str           { ~"TAY"       }
    fn txa(&mut self) -> ~str           { ~"TXA"       }
    fn tya(&mut self) -> ~str           { ~"TYA"       }
    fn txs(&mut self) -> ~str           { ~"TXS"       }
    fn tsx(&mut self) -> ~str           { ~"TSX"       }

    // Flag operations
    fn clc(&mut self) -> ~str           { ~"CLC"       }
    fn sec(&mut self) -> ~str           { ~"SEC"       }
    fn cli(&mut self) -> ~str           { ~"CLI"       }
    fn sei(&mut self) -> ~str           { ~"SEI"       }
    fn clv(&mut self) -> ~str           { ~"CLV"       }
    fn cld(&mut self) -> ~str           { ~"CLD"       }
    fn sed(&mut self) -> ~str           { ~"SED"       }

    // Branches
    // FIXME: Should disassemble the displacement!
    fn bpl(&mut self) -> ~str           { ~"BPL xx"    }
    fn bmi(&mut self) -> ~str           { ~"BMI xx"    }
    fn bvc(&mut self) -> ~str           { ~"BVC xx"    }
    fn bvs(&mut self) -> ~str           { ~"BVS xx"    }
    fn bcc(&mut self) -> ~str           { ~"BCC xx"    }
    fn bcs(&mut self) -> ~str           { ~"BCS xx"    }
    fn bne(&mut self) -> ~str           { ~"BNE xx"    }
    fn beq(&mut self) -> ~str           { ~"BEQ xx"    }

    // Jumps
    // FIXME: Should disassemble the address!
    fn jmp(&mut self) -> ~str           { ~"JMP xx"    }
    fn jmpi(&mut self) -> ~str          { ~"JMP (xx)"  }

    // Procedure calls
    // FIXME: Should disassemble the address!
    fn jsr(&mut self) -> ~str           { ~"JSR xx"    }
    fn rts(&mut self) -> ~str           { ~"RTS"       }
    fn brk(&mut self) -> ~str           { ~"BRK"       }
    fn rti(&mut self) -> ~str           { ~"RTI"       }

    // Stack operations
    fn pha(&mut self) -> ~str           { ~"PHA"       }
    fn pla(&mut self) -> ~str           { ~"PLA"       }
    fn php(&mut self) -> ~str           { ~"PHP"       }
    fn plp(&mut self) -> ~str           { ~"PLP"       }

    // No operation
    fn nop(&mut self) -> ~str           { ~"NOP"       }

    // Addressing modes
    fn immediate(&mut self) -> ~str          { ~"#" + self.disb_bump_pc()          }
    fn accumulator(&mut self) -> ~str        { ~""                                 }
    fn zero_page(&mut self) -> ~str          { self.disb_bump_pc()                 }
    fn zero_page_x(&mut self) -> ~str        { self.disb_bump_pc() + ~",X"         }
    fn zero_page_y(&mut self) -> ~str        { self.disb_bump_pc() + ~",Y"         }
    fn absolute(&mut self) -> ~str           { self.disw_bump_pc()                 }
    fn absolute_x(&mut self) -> ~str         { self.disw_bump_pc() + ~",X"         }
    fn absolute_y(&mut self) -> ~str         { self.disw_bump_pc() + ~",Y"         }
    fn indexed_indirect_x(&mut self) -> ~str { ~"(" + self.disb_bump_pc() + ~",X)" }
    fn indirect_indexed_y(&mut self) -> ~str { ~"(" + self.disb_bump_pc() + ~"),Y" }

    // The main disassembly routine.
    #[inline(never)]
    pub fn disassemble(&mut self) -> ~str {
        let op = self.loadb_bump_pc();
        decode_op!(
            op: op,
            this: self,
            modes: [
                self.immediate(),
                self.accumulator(),
                self.zero_page(),
                self.zero_page_x(),
                self.zero_page_y(),
                self.absolute(),
                self.absolute_x(),
                self.absolute_y(),
                self.indexed_indirect_x(),
                self.indirect_indexed_y()
            ]
        )
    }
}

