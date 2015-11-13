use RegData;
use AddressSpace;

pub struct Instr {
    opcode: u8,
    data: Vec<u8>,
    cycles: u32,
}

impl Instr {

    fn param_two(reg: &mut RegData, rom: &AddressSpace) -> Vec<u8> {
        let one = rom.read(reg.advance_pc());
        let two = rom.read(reg.advance_pc());
        vec![one, two]
    }

    fn param_one(reg: &mut RegData, rom: &AddressSpace) -> Vec<u8> {
        let one = rom.read(reg.advance_pc());
        vec![one]
    }

    pub fn parse(reg: &mut RegData, rom: &AddressSpace) -> Instr {
        let opcode = rom.read(reg.advance_pc());
        let (vec, cycles) = match opcode {
            // LD reg, immed
            0x06 | 0x0E | 0x16 | 0x1E | 0x26 | 0x2E | 0x3E => {
                (
                    Instr::param_one(reg, rom),
                    8
                )
            },
            // LD reg, reg
            0x78...0x7D | 0x7F | 0x40...0x45 | 0x47...0x4D | 0x4F | 0x50...0x55 | 0x57...0x5D | 0x5F
                | 0x60...0x65 | 0x67...0x6D | 0x6F => {
                (
                    Vec::new(),
                    4
                )
            },
            // LD reg, (reg) or LD (reg), reg
            0x7E | 0x46 | 0x4E | 0x56 | 0x5E | 0x66 | 0x6E | 0x70...0x75 | 0x0A | 0x1A | 0x02 | 0x12
                | 0x77 | 0xF2 | 0xE2 | 0x3A | 0x32 | 0x2A | 0x22 | 0xF9 => {
                (
                    Vec::new(),
                    8
                )
            },
            // LD (HL), immed or LD (FF00+immed), reg or vice versa
            // LDHL SP, immed
            0x36 | 0xE0 | 0xF0 | 0xF8 => {
                (
                    Instr::param_one(reg, rom),
                    12
                )
            },
            // LD A, (immed) or LD (immed), A
            0xFA | 0xEA => {
                (
                    Instr::param_two(reg, rom),
                    16
                )
            },
            // LD reg16, immed
            0x01 | 0x11 | 0x21 | 0x31 => {
                (
                    Instr::param_two(reg, rom),
                    12
                )
            },
            // LD (immed), SP
            0x08 => {
                (
                    Instr::param_two(reg, rom),
                    20
                )
            },
            // PUSH reg
            0xF5 | 0xC5 | 0xD5 | 0xE5 => {
                (
                    Vec::new(),
                    16
                )
            },
            // POP reg
            0xF1 | 0xC1 | 0xD1 | 0xE1 => {
                (
                    Vec::new(),
                    12
                )
            },
            // [ALU] reg, reg
            0x80...0x85 | 0x87...0x8D | 0x8F...0x95 | 0x97...0x9D | 0x9F...0xA5 | 0xA7...0xAD
                | 0xAF...0xB5 | 0xB7...0xBD | 0xBF => {
                (
                    Vec::new(),
                    4
                )
            },
            // [ALU] reg, (HL)
            0x86 | 0x8E | 0x96 | 0x9E | 0xA6 | 0xAE | 0xB6 | 0xBE => {
                (
                    Vec::new(),
                    8
                )
            },
            // [ALU] reg, immed
            0xC6 | 0xCE | 0xD6 | 0xDE | 0xE6 | 0xEE | 0xF6 | 0xFE => {
                (
                    Instr::param_one(reg, rom),
                    8
                )
            },
            // INC reg
            // DEC reg
            0x3C | 0x3D | 0x04 | 0x05 | 0x0C | 0x0D | 0x14 | 0x15 | 0x1C | 0x1D | 0x24 | 0x25 | 0x2C | 0x2D => {
                (
                    Vec::new(),
                    4
                )
            },
            // INC (HL)
            // DEC (HL)
            0x34 | 0x35 => {
                (
                    Vec::new(),
                    12
                )
            },
            // [ALU 16-bit] reg, reg
            // [ALU 16-bit] reg
            0x09 | 0x19 | 0x29 | 0x39 | 0x03 | 0x13 | 0x23 | 0x33 | 0x0B | 0x1B | 0x2B | 0x3B => {
                (
                    Vec::new(),
                    8
                )
            },
            // ADD SP, immed
            0xE8 => {
                (
                    Instr::param_one(reg, rom),
                    16
                )
            },
            // Bit instructions
            0xCB => {
                let v = Instr::param_one(reg, rom);
                let c = match v[0] {
                    // [bitop] reg
                    0x30...0x35 | 0x37 | 0x00...0x05 | 0x07...0x0D | 0x0F...0x15 | 0x17...0x1D
                        | 0x1F...0x25 | 0x27...0x2D | 0x2F | 0x38...0x3D | 0x3F...0x45 | 0x47 
                        | 0xC0...0xC5 | 0xC7 | 0x80...0x85 | 0x87 => {
                        8
                    },
                    // [bitop] (HL)
                    0x36 | 0x06 | 0x0E | 0x16 | 0x1E | 0x26 | 0x2E | 0x3E | 0x46 | 0xC6 | 0x86 => {
                        16
                    },

                    _ => {
                        println!("Warning: Missing instruction data for opcode {:X} {:X}", opcode, v[0]);
                        4
                    }
                };
                (v, c)
            },
            // DAA, CPL, CCF, SCF, NOP, HALT, DI, EI, RLCA, RLA, RRCA, RRA
            0x27 | 0x2F | 0x3F | 0x37 | 0x00 | 0x76 | 0xF3 | 0xFB | 0x07 | 0x17 | 0x0F | 0x1F => {
                (
                    Vec::new(),
                    4
                )
            },
            // STOP
            0x10 => {
                (
                    Instr::param_one(reg, rom),
                    4
                )
            },
            // JP cond, immed
            0xC3 | 0xC2 | 0xCA | 0xD2 | 0xDA => {
                (
                    Instr::param_two(reg, rom),
                    12
                )
            },
            // JP HL
            0xE9 => {
                (
                    Vec::new(),
                    4
                )
            },
            // JR cond, immed
            0x18 | 0x20 | 0x28 | 0x30 | 0x38 => {
                (
                    Instr::param_one(reg, rom),
                    8
                )
            },
            // CALL cond, immed
            0xCD | 0xC4 | 0xCC | 0xD4 | 0xDC => {
                (
                    Instr::param_two(reg, rom),
                    12
                )
            },
            // RST
            0xC7 | 0xCF | 0xD7 |  0xDF | 0xE7 | 0xEF | 0xF7 | 0xFF => {
                (
                    Vec::new(),
                    32
                )
            },
            // RET cond
            // RETI
            0xC9 | 0xC0 | 0xC8 | 0xD0 | 0xD8 | 0xD9 => {
                (
                    Vec::new(),
                    8
                )
            },


            // Decode unrecognized instructions with default values
            _ => {
                println!("Warning: Missing instruction data for opcode {:X}", opcode);
                (
                    Vec::new(),
                    4
                )
            }
        };
        Instr {
            opcode: opcode,
            data: vec,
            cycles: cycles,
        }
    }

    pub fn opcode(&self) -> u8 {
        self.opcode
    }

    pub fn cycles(&self) -> u32 {
        self.cycles
    }

    pub fn param(&self, i: usize) -> u8 {
        self.data[i]
    }

    pub fn param_u16(&self, i: usize) -> u16 {
        (self.data[i] as u16) | ((self.data[i + 1] as u16) << 8)
    }

}
