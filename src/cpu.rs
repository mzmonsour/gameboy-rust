use instr::Instr;
use AddressSpace;
use Register;
use RegFlag;
use RegData;

use std::num::Wrapping;

static GB_FREQUENCY: u32 = 4194304;

#[derive(Copy, Clone)]
pub enum CpuState {
    Running, // Instructions run normally
    Halted, // No instructions are run, reset on interrupt
    Stopped, // No instructions are run, reset on user input
}

pub struct Cpu {
    reg: RegData,
    ram: AddressSpace,
    freq: u32,
    clock: u64,
    state: CpuState,
    intlevel: bool,
}

impl Cpu {

    pub fn new() -> Cpu {
        Cpu::new_with_freq(GB_FREQUENCY)
    }

    pub fn new_with_freq(freq: u32) -> Cpu {
        Cpu {
            reg: RegData::new(),
            ram: AddressSpace::new(),
            freq: freq,
            clock: 0,
            state: CpuState::Running,
            intlevel: true,
        }
    }

    pub fn is_stopped(self) -> bool {
        if let CpuState::Stopped = self.state {
            true
        } else {
            false
        }
    }

    pub fn do_instr(&mut self) -> u32 {
        match self.state {
            CpuState::Running => (),
            CpuState::Halted | CpuState::Stopped => {
                if !self.intlevel {
                    println!("Warning: CPU stopped/halted and interrupts are disabled!");
                }
                return 0;
            }
        }
        let instr = Instr::parse(&mut self.reg, &self.ram);
        match instr.opcode() {
            // 8-bit immediate loads
            0x06 => self.reg.write(Register::B, instr.param(0)),
            0x0E => self.reg.write(Register::C, instr.param(0)),
            0x16 => self.reg.write(Register::D, instr.param(0)),
            0x1E => self.reg.write(Register::E, instr.param(0)),
            0x26 => self.reg.write(Register::H, instr.param(0)),
            0x2E => self.reg.write(Register::L, instr.param(0)),
            // 8-bit register loads
            0x47 => self.reg.copy(Register::B, Register::A),
            0x40 => self.reg.copy(Register::B, Register::B),
            0x41 => self.reg.copy(Register::B, Register::C),
            0x42 => self.reg.copy(Register::B, Register::D),
            0x43 => self.reg.copy(Register::B, Register::E),
            0x44 => self.reg.copy(Register::B, Register::H),
            0x45 => self.reg.copy(Register::B, Register::L),
            0x46 => {
                let addr = self.reg.read_u16(Register::HL);
                self.reg.write(Register::B, self.ram.read(addr));
            },
            0x4F => self.reg.copy(Register::C, Register::A),
            0x48 => self.reg.copy(Register::C, Register::B),
            0x49 => self.reg.copy(Register::C, Register::C),
            0x4A => self.reg.copy(Register::C, Register::D),
            0x4B => self.reg.copy(Register::C, Register::E),
            0x4C => self.reg.copy(Register::C, Register::H),
            0x4D => self.reg.copy(Register::C, Register::L),
            0x4E => {
                let addr = self.reg.read_u16(Register::HL);
                self.reg.write(Register::C, self.ram.read(addr));
            },
            0x57 => self.reg.copy(Register::D, Register::A),
            0x50 => self.reg.copy(Register::D, Register::B),
            0x51 => self.reg.copy(Register::D, Register::C),
            0x52 => self.reg.copy(Register::D, Register::D),
            0x53 => self.reg.copy(Register::D, Register::E),
            0x54 => self.reg.copy(Register::D, Register::H),
            0x55 => self.reg.copy(Register::D, Register::L),
            0x56 => {
                let addr = self.reg.read_u16(Register::HL);
                self.reg.write(Register::D, self.ram.read(addr));
            },
            0x5F => self.reg.copy(Register::E, Register::A),
            0x58 => self.reg.copy(Register::E, Register::B),
            0x59 => self.reg.copy(Register::E, Register::C),
            0x5A => self.reg.copy(Register::E, Register::D),
            0x5B => self.reg.copy(Register::E, Register::E),
            0x5C => self.reg.copy(Register::E, Register::H),
            0x5D => self.reg.copy(Register::E, Register::L),
            0x5E => {
                let addr = self.reg.read_u16(Register::HL);
                self.reg.write(Register::E, self.ram.read(addr));
            },
            0x67 => self.reg.copy(Register::H, Register::A),
            0x60 => self.reg.copy(Register::H, Register::B),
            0x61 => self.reg.copy(Register::H, Register::C),
            0x62 => self.reg.copy(Register::H, Register::D),
            0x63 => self.reg.copy(Register::H, Register::E),
            0x64 => self.reg.copy(Register::H, Register::H),
            0x65 => self.reg.copy(Register::H, Register::L),
            0x66 => {
                let addr = self.reg.read_u16(Register::HL);
                self.reg.write(Register::H, self.ram.read(addr));
            },
            0x6F => self.reg.copy(Register::L, Register::A),
            0x68 => self.reg.copy(Register::L, Register::B),
            0x69 => self.reg.copy(Register::L, Register::C),
            0x6A => self.reg.copy(Register::L, Register::D),
            0x6B => self.reg.copy(Register::L, Register::E),
            0x6C => self.reg.copy(Register::L, Register::H),
            0x6D => self.reg.copy(Register::L, Register::L),
            0x6E => {
                let addr = self.reg.read_u16(Register::HL);
                self.reg.write(Register::L, self.ram.read(addr));
            },
            // 8-bit load to ram
            0x70 => self.ram.write(self.reg.read_u16(Register::HL), self.reg.read(Register::B)),
            0x71 => self.ram.write(self.reg.read_u16(Register::HL), self.reg.read(Register::C)),
            0x72 => self.ram.write(self.reg.read_u16(Register::HL), self.reg.read(Register::D)),
            0x73 => self.ram.write(self.reg.read_u16(Register::HL), self.reg.read(Register::E)),
            0x74 => self.ram.write(self.reg.read_u16(Register::HL), self.reg.read(Register::H)),
            0x75 => self.ram.write(self.reg.read_u16(Register::HL), self.reg.read(Register::L)),
            0x36 => self.ram.write(self.reg.read_u16(Register::HL), instr.param(0)),
            // loads into register A
            0x7F => self.reg.copy(Register::A, Register::A),
            0x78 => self.reg.copy(Register::A, Register::B),
            0x79 => self.reg.copy(Register::A, Register::C),
            0x7A => self.reg.copy(Register::A, Register::D),
            0x7B => self.reg.copy(Register::A, Register::E),
            0x7C => self.reg.copy(Register::A, Register::H),
            0x7D => self.reg.copy(Register::A, Register::L),
            0x0A => {
                let addr =self.reg.read_u16(Register::BC);
                self.reg.write(Register::A, self.ram.read(addr));
            },
            0x1A => {
                let addr =self.reg.read_u16(Register::DE);
                self.reg.write(Register::A, self.ram.read(addr));
            },
            0x7E => {
                let addr =self.reg.read_u16(Register::HL);
                self.reg.write(Register::A, self.ram.read(addr));
            },
            0xFA => self.reg.write(Register::A, self.ram.read(instr.param_u16(0))),
            0x3E => self.reg.write(Register::A, instr.param(0)),
            // writes from register A
            0x02 => self.ram.write(self.reg.read_u16(Register::BC), self.reg.read(Register::A)),
            0x12 => self.ram.write(self.reg.read_u16(Register::DE), self.reg.read(Register::A)),
            0x77 => self.ram.write(self.reg.read_u16(Register::HL), self.reg.read(Register::A)),
            0xEA => self.ram.write(instr.param_u16(0), self.reg.read(Register::A)),
            // Read/Write ($FF00 + C) with A
            0xF2 => {
                let addr = 0xFF00 + self.reg.read(Register::C) as u16;
                self.reg.write(Register::A, self.ram.read(addr));
            },
            0xE2 => self.ram.write(0xFF00 + self.reg.read(Register::C) as u16, self.reg.read(Register::A)),
            // Load from (HL) and decrement
            0x3A => {
                let addr = self.reg.read_u16(Register::HL);
                self.reg.write(Register::A, self.ram.read(addr));
                self.reg.write_u16(Register::HL, addr - 1);
            },
            // Write to (HL) and decrement
            0x32 => {
                let addr = self.reg.read_u16(Register::HL);
                self.ram.write(addr, self.reg.read(Register::A));
                self.reg.write_u16(Register::HL, addr - 1);
            },
            // Load from (HL) and increment
            0x2A => {
                let addr = self.reg.read_u16(Register::HL);
                self.reg.write(Register::A, self.ram.read(addr));
                self.reg.write_u16(Register::HL, addr + 1);
            },
            // Write to (HL) and increment
            0x22 => {
                let addr = self.reg.read_u16(Register::HL);
                self.ram.write(addr, self.reg.read(Register::A));
                self.reg.write_u16(Register::HL, addr + 1);
            },
            // Write to ($FF00 + immediate)
            0xE0 => self.ram.write(0xFF00 + instr.param(0) as u16, self.reg.read(Register::A)),
            // Load ($FF00 + immediate)
            0xF0 => self.reg.write(Register::A, self.ram.read(0xFF00 + instr.param(0) as u16)),
            // 16-bit immediate loads
            0x01 => self.reg.write_u16(Register::BC, instr.param_u16(0)),
            0x11 => self.reg.write_u16(Register::DE, instr.param_u16(0)),
            0x21 => self.reg.write_u16(Register::HL, instr.param_u16(0)),
            0x31 => self.reg.write_u16(Register::SP, instr.param_u16(0)),
            // Copy HL into SP
            0xF9 => self.reg.copy_u16(Register::SP, Register::HL),
            // Load effective address SP + immediate into HL
            0xF8 => {
                let addr = self.reg.read_u16(Register::SP) + instr.param(0) as u16;
                self.reg.write_u16(Register::HL, addr);
            },
            // Put SP at (immediate)
            0x08 => self.ram.write_u16(instr.param_u16(0), self.reg.read_u16(Register::SP)),
            // Push instructions
            0xF5 => {
                let addr = self.reg.read_u16(Register::SP);
                self.ram.write_u16(addr, self.reg.read_u16(Register::AF));
                self.reg.write_u16(Register::SP, addr - 2);
            },
            0xC5 => {
                let addr = self.reg.read_u16(Register::SP);
                self.ram.write_u16(addr, self.reg.read_u16(Register::BC));
                self.reg.write_u16(Register::SP, addr - 2);
            },
            0xD5 => {
                let addr = self.reg.read_u16(Register::SP);
                self.ram.write_u16(addr, self.reg.read_u16(Register::DE));
                self.reg.write_u16(Register::SP, addr - 2);
            },
            0xE5 => {
                let addr = self.reg.read_u16(Register::SP);
                self.ram.write_u16(addr, self.reg.read_u16(Register::HL));
                self.reg.write_u16(Register::SP, addr - 2);
            },
            // Pop instructions
            0xF1 => {
                let addr = self.reg.read_u16(Register::SP);
                self.reg.write_u16(Register::AF, self.ram.read_u16(addr));
                self.reg.write_u16(Register::SP, addr + 2);
            },
            0xC1 => {
                let addr = self.reg.read_u16(Register::SP);
                self.reg.write_u16(Register::BC, self.ram.read_u16(addr));
                self.reg.write_u16(Register::SP, addr + 2);
            },
            0xD1 => {
                let addr = self.reg.read_u16(Register::SP);
                self.reg.write_u16(Register::DE, self.ram.read_u16(addr));
                self.reg.write_u16(Register::SP, addr + 2);
            },
            0xE1 => {
                let addr = self.reg.read_u16(Register::SP);
                self.reg.write_u16(Register::HL, self.ram.read_u16(addr));
                self.reg.write_u16(Register::SP, addr + 2);
            },
            // Add instructions
            0x87 => {
                let n = self.reg.read(Register::A);
                self.add(Register::A, n);
            },
            0x80 => {
                let n = self.reg.read(Register::B);
                self.add(Register::A, n);
            },
            0x81 => {
                let n = self.reg.read(Register::C);
                self.add(Register::A, n);
            },
            0x82 => {
                let n = self.reg.read(Register::D);
                self.add(Register::A, n);
            },
            0x83 => {
                let n = self.reg.read(Register::E);
                self.add(Register::A, n);
            },
            0x84 => {
                let n = self.reg.read(Register::H);
                self.add(Register::A, n);
            },
            0x85 => {
                let n = self.reg.read(Register::L);
                self.add(Register::A, n);
            },
            0x86 => {
                let n = self.ram.read(self.reg.read_u16(Register::HL));
                self.add(Register::A, n);
            },
            0xC6 => {
                let n = instr.param(0);
                self.add(Register::A, n);
            },
            // Add with carry instructions
            0x8F => {
                let carry = self.reg.get_flag(RegFlag::Carry);
                let n = self.reg.read(Register::A);
                self.add_with_carry(Register::A, n, carry);
            },
            0x88 => {
                let carry = self.reg.get_flag(RegFlag::Carry);
                let n = self.reg.read(Register::B);
                self.add_with_carry(Register::A, n, carry);
            },
            0x89 => {
                let carry = self.reg.get_flag(RegFlag::Carry);
                let n = self.reg.read(Register::C);
                self.add_with_carry(Register::A, n, carry);
            },
            0x8A => {
                let carry = self.reg.get_flag(RegFlag::Carry);
                let n = self.reg.read(Register::D);
                self.add_with_carry(Register::A, n, carry);
            },
            0x8B => {
                let carry = self.reg.get_flag(RegFlag::Carry);
                let n = self.reg.read(Register::E);
                self.add_with_carry(Register::A, n, carry);
            },
            0x8C => {
                let carry = self.reg.get_flag(RegFlag::Carry);
                let n = self.reg.read(Register::H);
                self.add_with_carry(Register::A, n, carry);
            },
            0x8D => {
                let carry = self.reg.get_flag(RegFlag::Carry);
                let n = self.reg.read(Register::L);
                self.add_with_carry(Register::A, n, carry);
            },
            0x8E => {
                let carry = self.reg.get_flag(RegFlag::Carry);
                let n = self.ram.read(self.reg.read_u16(Register::HL));
                self.add_with_carry(Register::A, n, carry);
            },
            0xCE => {
                let carry = self.reg.get_flag(RegFlag::Carry);
                let n = instr.param(0);
                self.add_with_carry(Register::A, n, carry);
            },
            // Subtract
            0x97 => {
                let n = self.reg.read(Register::A);
                self.sub(Register::A, n);
            },
            0x90 => {
                let n = self.reg.read(Register::B);
                self.sub(Register::A, n);
            },
            0x91 => {
                let n = self.reg.read(Register::C);
                self.sub(Register::A, n);
            },
            0x92 => {
                let n = self.reg.read(Register::D);
                self.sub(Register::A, n);
            },
            0x93 => {
                let n = self.reg.read(Register::E);
                self.sub(Register::A, n);
            },
            0x94 => {
                let n = self.reg.read(Register::H);
                self.sub(Register::A, n);
            },
            0x95 => {
                let n = self.reg.read(Register::L);
                self.sub(Register::A, n);
            },
            0x96 => {
                let n = self.ram.read(self.reg.read_u16(Register::HL));
                self.sub(Register::A, n);
            },
            0xD6 => {
                let n = instr.param(0);
                self.sub(Register::A, n);
            },
            // Subtract with carry
            0x9F => {
                let carry = self.reg.get_flag(RegFlag::Carry);
                let n = self.reg.read(Register::A);
                self.sub_with_carry(Register::A, n, carry);
            },
            0x98 => {
                let carry = self.reg.get_flag(RegFlag::Carry);
                let n = self.reg.read(Register::B);
                self.sub_with_carry(Register::A, n, carry);
            },
            0x99 => {
                let carry = self.reg.get_flag(RegFlag::Carry);
                let n = self.reg.read(Register::C);
                self.sub_with_carry(Register::A, n, carry);
            },
            0x9A => {
                let carry = self.reg.get_flag(RegFlag::Carry);
                let n = self.reg.read(Register::D);
                self.sub_with_carry(Register::A, n, carry);
            },
            0x9B => {
                let carry = self.reg.get_flag(RegFlag::Carry);
                let n = self.reg.read(Register::E);
                self.sub_with_carry(Register::A, n, carry);
            },
            0x9C => {
                let carry = self.reg.get_flag(RegFlag::Carry);
                let n = self.reg.read(Register::H);
                self.sub_with_carry(Register::A, n, carry);
            },
            0x9D => {
                let carry = self.reg.get_flag(RegFlag::Carry);
                let n = self.reg.read(Register::L);
                self.sub_with_carry(Register::A, n, carry);
            },
            0x9E => {
                let carry = self.reg.get_flag(RegFlag::Carry);
                let n = self.ram.read(self.reg.read_u16(Register::HL));
                self.sub_with_carry(Register::A, n, carry);
            },
            0xDE => {
                let carry = self.reg.get_flag(RegFlag::Carry);
                let n = instr.param(0);
                self.sub_with_carry(Register::A, n, carry);
            },
            // Bitwise AND
            0xA7 => {
                let a = self.reg.read(Register::A);
                let n = a;
                let x = a & n;
                self.set_bitand_flags(x);
                self.reg.write(Register::A, x);
            },
            0xA0 => {
                let a = self.reg.read(Register::A);
                let n = self.reg.read(Register::B);
                let x = a & n;
                self.set_bitand_flags(x);
                self.reg.write(Register::A, x);
            },
            0xA1 => {
                let a = self.reg.read(Register::A);
                let n = self.reg.read(Register::C);
                let x = a & n;
                self.set_bitand_flags(x);
                self.reg.write(Register::A, x);
            },
            0xA2 => {
                let a = self.reg.read(Register::A);
                let n = self.reg.read(Register::D);
                let x = a & n;
                self.set_bitand_flags(x);
                self.reg.write(Register::A, x);
            },
            0xA3 => {
                let a = self.reg.read(Register::A);
                let n = self.reg.read(Register::E);
                let x = a & n;
                self.set_bitand_flags(x);
                self.reg.write(Register::A, x);
            },
            0xA4 => {
                let a = self.reg.read(Register::A);
                let n = self.reg.read(Register::H);
                let x = a & n;
                self.set_bitand_flags(x);
                self.reg.write(Register::A, x);
            },
            0xA5 => {
                let a = self.reg.read(Register::A);
                let n = self.reg.read(Register::L);
                let x = a & n;
                self.set_bitand_flags(x);
                self.reg.write(Register::A, x);
            },
            0xA6 => {
                let a = self.reg.read(Register::A);
                let n = self.ram.read(self.reg.read_u16(Register::HL));
                let x = a & n;
                self.set_bitand_flags(x);
                self.reg.write(Register::A, x);
            },
            0xE6 => {
                let a = self.reg.read(Register::A);
                let n = instr.param(0);
                let x = a & n;
                self.set_bitand_flags(x);
                self.reg.write(Register::A, x);
            },
            // Bitwise OR
            0xB7 => {
                let a = self.reg.read(Register::A);
                let n = a;
                let x = a | n;
                self.set_bitor_flags(x);
                self.reg.write(Register::A, x);
            },
            0xB0 => {
                let a = self.reg.read(Register::A);
                let n = self.reg.read(Register::B);
                let x = a | n;
                self.set_bitor_flags(x);
                self.reg.write(Register::A, x);
            },
            0xB1 => {
                let a = self.reg.read(Register::A);
                let n = self.reg.read(Register::C);
                let x = a | n;
                self.set_bitor_flags(x);
                self.reg.write(Register::A, x);
            },
            0xB2 => {
                let a = self.reg.read(Register::A);
                let n = self.reg.read(Register::D);
                let x = a | n;
                self.set_bitor_flags(x);
                self.reg.write(Register::A, x);
            },
            0xB3 => {
                let a = self.reg.read(Register::A);
                let n = self.reg.read(Register::E);
                let x = a | n;
                self.set_bitor_flags(x);
                self.reg.write(Register::A, x);
            },
            0xB4 => {
                let a = self.reg.read(Register::A);
                let n = self.reg.read(Register::H);
                let x = a | n;
                self.set_bitor_flags(x);
                self.reg.write(Register::A, x);
            },
            0xB5 => {
                let a = self.reg.read(Register::A);
                let n = self.reg.read(Register::L);
                let x = a | n;
                self.set_bitor_flags(x);
                self.reg.write(Register::A, x);
            },
            0xB6 => {
                let a = self.reg.read(Register::A);
                let n = self.ram.read(self.reg.read_u16(Register::HL));
                let x = a | n;
                self.set_bitor_flags(x);
                self.reg.write(Register::A, x);
            },
            0xF6 => {
                let a = self.reg.read(Register::A);
                let n = instr.param(0);
                let x = a | n;
                self.set_bitor_flags(x);
                self.reg.write(Register::A, x);
            },
            // Bitwise XOR
            0xAF => {
                let a = self.reg.read(Register::A);
                let n = a;
                let x = a ^ n;
                self.set_bitor_flags(x);
                self.reg.write(Register::A, x);
            },
            0xA8 => {
                let a = self.reg.read(Register::A);
                let n = self.reg.read(Register::B);
                let x = a ^ n;
                self.set_bitor_flags(x);
                self.reg.write(Register::A, x);
            },
            0xA9 => {
                let a = self.reg.read(Register::A);
                let n = self.reg.read(Register::C);
                let x = a ^ n;
                self.set_bitor_flags(x);
                self.reg.write(Register::A, x);
            },
            0xAA => {
                let a = self.reg.read(Register::A);
                let n = self.reg.read(Register::D);
                let x = a ^ n;
                self.set_bitor_flags(x);
                self.reg.write(Register::A, x);
            },
            0xAB => {
                let a = self.reg.read(Register::A);
                let n = self.reg.read(Register::E);
                let x = a ^ n;
                self.set_bitor_flags(x);
                self.reg.write(Register::A, x);
            },
            0xAC => {
                let a = self.reg.read(Register::A);
                let n = self.reg.read(Register::H);
                let x = a ^ n;
                self.set_bitor_flags(x);
                self.reg.write(Register::A, x);
            },
            0xAD => {
                let a = self.reg.read(Register::A);
                let n = self.reg.read(Register::L);
                let x = a ^ n;
                self.set_bitor_flags(x);
                self.reg.write(Register::A, x);
            },
            0xAE => {
                let a = self.reg.read(Register::A);
                let n = self.ram.read(self.reg.read_u16(Register::HL));
                let x = a ^ n;
                self.set_bitor_flags(x);
                self.reg.write(Register::A, x);
            },
            0xEE => {
                let a = self.reg.read(Register::A);
                let n = instr.param(0);
                let x = a ^ n;
                self.set_bitor_flags(x);
                self.reg.write(Register::A, x);
            },
            // Comparison instructions
            0xBF => {
                let a = self.reg.read(Register::A);
                let n = a;
                self.sub_no_writeback(a, n, false);
            }
            0xB8 => {
                let a = self.reg.read(Register::A);
                let n = self.reg.read(Register::B);
                self.sub_no_writeback(a, n, false);
            }
            0xB9 => {
                let a = self.reg.read(Register::A);
                let n = self.reg.read(Register::C);
                self.sub_no_writeback(a, n, false);
            }
            0xBA => {
                let a = self.reg.read(Register::A);
                let n = self.reg.read(Register::D);
                self.sub_no_writeback(a, n, false);
            }
            0xBB => {
                let a = self.reg.read(Register::A);
                let n = self.reg.read(Register::E);
                self.sub_no_writeback(a, n, false);
            }
            0xBC => {
                let a = self.reg.read(Register::A);
                let n = self.reg.read(Register::H);
                self.sub_no_writeback(a, n, false);
            }
            0xBD => {
                let a = self.reg.read(Register::A);
                let n = self.reg.read(Register::L);
                self.sub_no_writeback(a, n, false);
            }
            0xBE => {
                let a = self.reg.read(Register::A);
                let n = self.ram.read(self.reg.read_u16(Register::HL));
                self.sub_no_writeback(a, n, false);
            }
            0xFE => {
                let a = self.reg.read(Register::A);
                let n = instr.param(0);
                self.sub_no_writeback(a, n, false);
            }
            // Incrementing
            0x3C => self.add(Register::A, 1),
            0x04 => self.add(Register::B, 1),
            0x0C => self.add(Register::C, 1),
            0x14 => self.add(Register::D, 1),
            0x1C => self.add(Register::E, 1),
            0x24 => self.add(Register::H, 1),
            0x2C => self.add(Register::L, 1),
            0x34 => {
                let addr = self.reg.read_u16(Register::HL);
                let n = self.ram.read(addr);
                let sum = self.add_no_writeback(n, 1, false);
                self.ram.write(addr, sum);
            }
            // Decrementing
            0x3D => self.sub(Register::A, 1),
            0x05 => self.sub(Register::B, 1),
            0x0D => self.sub(Register::C, 1),
            0x15 => self.sub(Register::D, 1),
            0x1D => self.sub(Register::E, 1),
            0x25 => self.sub(Register::H, 1),
            0x2D => self.sub(Register::L, 1),
            0x35 => {
                let addr = self.reg.read_u16(Register::HL);
                let n = self.ram.read(addr);
                let diff = self.sub_no_writeback(n, 1, false);
                self.ram.write(addr, diff);
            }
            // 16-bit add
            0x09 => {
                let n = self.reg.read_u16(Register::BC);
                self.add_u16(Register::HL, n);
            },
            0x19 => {
                let n = self.reg.read_u16(Register::DE);
                self.add_u16(Register::HL, n);
            },
            0x29 => {
                let n = self.reg.read_u16(Register::HL);
                self.add_u16(Register::HL, n);
            },
            0x39 => {
                let n = self.reg.read_u16(Register::SP);
                self.add_u16(Register::HL, n);
            },
            0xE8 => {
                let n = instr.param(0) as u16;
                self.add_u16(Register::SP, n);
            },
            // 16-bit increment
            // May not affect CPU flags?
            0x03 => self.add_u16(Register::BC, 1),
            0x13 => self.add_u16(Register::DE, 1),
            0x23 => self.add_u16(Register::HL, 1),
            0x33 => self.add_u16(Register::SP, 1),
            // 16-bit decrement
            0x0B => self.dec_u16(Register::BC),
            0x1B => self.dec_u16(Register::DE),
            0x2B => self.dec_u16(Register::HL),
            0x3B => self.dec_u16(Register::SP),
            // Bit operations
            0xCB => {
                match instr.param(0) {
                    // Swap "upper and lower nibbles"
                    0x37 => self.swap_bits(Register::A),
                    0x30 => self.swap_bits(Register::B),
                    0x31 => self.swap_bits(Register::C),
                    0x32 => self.swap_bits(Register::D),
                    0x33 => self.swap_bits(Register::E),
                    0x34 => self.swap_bits(Register::H),
                    0x35 => self.swap_bits(Register::L),
                    0x36 => {
                        let addr = self.reg.read_u16(Register::HL);
                        let n = self.ram.read(addr);
                        let x = self.swap_bits_no_writeback(n);
                        self.ram.write(addr, x);
                    },
                    // Rotate left
                    0x07 => {
                        let x = self.reg.read(Register::A);
                        let rot = self.lrot(x);
                        self.reg.write(Register::A, rot);
                    },
                    0x00 => {
                        let x = self.reg.read(Register::B);
                        let rot = self.lrot(x);
                        self.reg.write(Register::B, rot);
                    },
                    0x01 => {
                        let x = self.reg.read(Register::C);
                        let rot = self.lrot(x);
                        self.reg.write(Register::C, rot);
                    },
                    0x02 => {
                        let x = self.reg.read(Register::D);
                        let rot = self.lrot(x);
                        self.reg.write(Register::D, rot);
                    },
                    0x03 => {
                        let x = self.reg.read(Register::E);
                        let rot = self.lrot(x);
                        self.reg.write(Register::E, rot);
                    },
                    0x04 => {
                        let x = self.reg.read(Register::H);
                        let rot = self.lrot(x);
                        self.reg.write(Register::H, rot);
                    },
                    0x05 => {
                        let x = self.reg.read(Register::L);
                        let rot = self.lrot(x);
                        self.reg.write(Register::L, rot);
                    },
                    0x06 => {
                        let addr = self.reg.read_u16(Register::HL);
                        let x = self.ram.read(addr);
                        let rot = self.lrot(x);
                        self.ram.write(addr, rot);
                    },
                    // Rotate left through carry
                    0x17 => {
                        let x = self.reg.read(Register::A);
                        let rot = self.lrot_through(x);
                        self.reg.write(Register::A, rot);
                    },
                    0x10 => {
                        let x = self.reg.read(Register::B);
                        let rot = self.lrot_through(x);
                        self.reg.write(Register::B, rot);
                    },
                    0x11 => {
                        let x = self.reg.read(Register::C);
                        let rot = self.lrot_through(x);
                        self.reg.write(Register::C, rot);
                    },
                    0x12 => {
                        let x = self.reg.read(Register::D);
                        let rot = self.lrot_through(x);
                        self.reg.write(Register::D, rot);
                    },
                    0x13 => {
                        let x = self.reg.read(Register::E);
                        let rot = self.lrot_through(x);
                        self.reg.write(Register::E, rot);
                    },
                    0x14 => {
                        let x = self.reg.read(Register::H);
                        let rot = self.lrot_through(x);
                        self.reg.write(Register::H, rot);
                    },
                    0x15 => {
                        let x = self.reg.read(Register::L);
                        let rot = self.lrot_through(x);
                        self.reg.write(Register::L, rot);
                    },
                    0x16 => {
                        let addr = self.reg.read_u16(Register::HL);
                        let x = self.ram.read(addr);
                        let rot = self.lrot_through(x);
                        self.ram.write(addr, rot);
                    },
                    // Rotate right
                    0x0F => {
                        let x = self.reg.read(Register::A);
                        let rot = self.rrot(x);
                        self.reg.write(Register::A, rot);
                    },
                    0x08 => {
                        let x = self.reg.read(Register::B);
                        let rot = self.rrot(x);
                        self.reg.write(Register::B, rot);
                    },
                    0x09 => {
                        let x = self.reg.read(Register::C);
                        let rot = self.rrot(x);
                        self.reg.write(Register::C, rot);
                    },
                    0x0A => {
                        let x = self.reg.read(Register::D);
                        let rot = self.rrot(x);
                        self.reg.write(Register::D, rot);
                    },
                    0x0B => {
                        let x = self.reg.read(Register::E);
                        let rot = self.rrot(x);
                        self.reg.write(Register::E, rot);
                    },
                    0x0C => {
                        let x = self.reg.read(Register::H);
                        let rot = self.rrot(x);
                        self.reg.write(Register::H, rot);
                    },
                    0x0D => {
                        let x = self.reg.read(Register::L);
                        let rot = self.rrot(x);
                        self.reg.write(Register::L, rot);
                    },
                    0x0E => {
                        let addr = self.reg.read_u16(Register::HL);
                        let x = self.ram.read(addr);
                        let rot = self.rrot(x);
                        self.ram.write(addr, rot);
                    },
                    // Rotate right through carry
                    0x1F => {
                        let x = self.reg.read(Register::A);
                        let rot = self.rrot_through(x);
                        self.reg.write(Register::A, rot);
                    },
                    0x18 => {
                        let x = self.reg.read(Register::B);
                        let rot = self.rrot_through(x);
                        self.reg.write(Register::B, rot);
                    },
                    0x19 => {
                        let x = self.reg.read(Register::C);
                        let rot = self.rrot_through(x);
                        self.reg.write(Register::C, rot);
                    },
                    0x1A => {
                        let x = self.reg.read(Register::D);
                        let rot = self.rrot_through(x);
                        self.reg.write(Register::D, rot);
                    },
                    0x1B => {
                        let x = self.reg.read(Register::E);
                        let rot = self.rrot_through(x);
                        self.reg.write(Register::E, rot);
                    },
                    0x1C => {
                        let x = self.reg.read(Register::H);
                        let rot = self.rrot_through(x);
                        self.reg.write(Register::H, rot);
                    },
                    0x1D => {
                        let x = self.reg.read(Register::L);
                        let rot = self.rrot_through(x);
                        self.reg.write(Register::L, rot);
                    },
                    0x1E => {
                        let addr = self.reg.read_u16(Register::HL);
                        let x = self.ram.read(addr);
                        let rot = self.rrot_through(x);
                        self.ram.write(addr, rot);
                    },
                    // Shift left
                    0x27 => {
                        let x = self.reg.read(Register::A);
                        let shift = self.lshift(x);
                        self.reg.write(Register::A, shift);
                    },
                    0x20 => {
                        let x = self.reg.read(Register::B);
                        let shift = self.lshift(x);
                        self.reg.write(Register::B, shift);
                    },
                    0x21 => {
                        let x = self.reg.read(Register::C);
                        let shift = self.lshift(x);
                        self.reg.write(Register::C, shift);
                    },
                    0x22 => {
                        let x = self.reg.read(Register::D);
                        let shift = self.lshift(x);
                        self.reg.write(Register::D, shift);
                    },
                    0x23 => {
                        let x = self.reg.read(Register::E);
                        let shift = self.lshift(x);
                        self.reg.write(Register::E, shift);
                    },
                    0x24 => {
                        let x = self.reg.read(Register::H);
                        let shift = self.lshift(x);
                        self.reg.write(Register::H, shift);
                    },
                    0x25 => {
                        let x = self.reg.read(Register::L);
                        let shift = self.lshift(x);
                        self.reg.write(Register::L, shift);
                    },
                    0x26 => {
                        let addr = self.reg.read_u16(Register::HL);
                        let x = self.ram.read(addr);
                        let shift = self.lshift(x);
                        self.ram.write(addr, shift);
                    },
                    // Shift right arithmetic
                    0x2F => {
                        let x = self.reg.read(Register::A);
                        let shift = self.rshift_arithmetic(x);
                        self.reg.write(Register::A, shift);
                    },
                    0x28 => {
                        let x = self.reg.read(Register::B);
                        let shift = self.rshift_arithmetic(x);
                        self.reg.write(Register::B, shift);
                    },
                    0x29 => {
                        let x = self.reg.read(Register::C);
                        let shift = self.rshift_arithmetic(x);
                        self.reg.write(Register::C, shift);
                    },
                    0x2A => {
                        let x = self.reg.read(Register::D);
                        let shift = self.rshift_arithmetic(x);
                        self.reg.write(Register::D, shift);
                    },
                    0x2B => {
                        let x = self.reg.read(Register::E);
                        let shift = self.rshift_arithmetic(x);
                        self.reg.write(Register::E, shift);
                    },
                    0x2C => {
                        let x = self.reg.read(Register::H);
                        let shift = self.rshift_arithmetic(x);
                        self.reg.write(Register::H, shift);
                    },
                    0x2D => {
                        let x = self.reg.read(Register::L);
                        let shift = self.rshift_arithmetic(x);
                        self.reg.write(Register::L, shift);
                    },
                    0x2E => {
                        let addr = self.reg.read_u16(Register::HL);
                        let x = self.ram.read(addr);
                        let shift = self.rshift_arithmetic(x);
                        self.ram.write(addr, shift);
                    },
                    // Shift right logical
                    0x3F => {
                        let x = self.reg.read(Register::A);
                        let shift = self.rshift_logical(x);
                        self.reg.write(Register::A, shift);
                    },
                    0x38 => {
                        let x = self.reg.read(Register::B);
                        let shift = self.rshift_logical(x);
                        self.reg.write(Register::B, shift);
                    },
                    0x39 => {
                        let x = self.reg.read(Register::C);
                        let shift = self.rshift_logical(x);
                        self.reg.write(Register::C, shift);
                    },
                    0x3A => {
                        let x = self.reg.read(Register::D);
                        let shift = self.rshift_logical(x);
                        self.reg.write(Register::D, shift);
                    },
                    0x3B => {
                        let x = self.reg.read(Register::E);
                        let shift = self.rshift_logical(x);
                        self.reg.write(Register::E, shift);
                    },
                    0x3C => {
                        let x = self.reg.read(Register::H);
                        let shift = self.rshift_logical(x);
                        self.reg.write(Register::H, shift);
                    },
                    0x3D => {
                        let x = self.reg.read(Register::L);
                        let shift = self.rshift_logical(x);
                        self.reg.write(Register::L, shift);
                    },
                    0x3E => {
                        let addr = self.reg.read_u16(Register::HL);
                        let x = self.ram.read(addr);
                        let shift = self.rshift_logical(x);
                        self.ram.write(addr, shift);
                    },

                    _ => panic!("Instruction not implemented! Opcode {:X} {:X}", instr.opcode(), instr.param(0)),
                }
            },
            // DAA instruction: See Z80 reference for behavior
            0x27 => {
                println!("Warning: Instruction DAA not implemented");
            },
            // Complement register
            0x2F => {
                let x = self.reg.read(Register::A) ^ 0xFF;
                self.reg.set_flag(RegFlag::Subtract, true);
                self.reg.set_flag(RegFlag::HalfCarry, true);
                self.reg.write(Register::A, x);
            },
            // Complement carry flag
            0x3F => {
                let c = self.reg.get_flag(RegFlag::Carry);
                self.reg.set_flag(RegFlag::Subtract, false);
                self.reg.set_flag(RegFlag::HalfCarry, false);
                self.reg.set_flag(RegFlag::Carry, !c);
            },
            // Set carry flag
            0x37 => {
                self.reg.set_flag(RegFlag::Subtract, false);
                self.reg.set_flag(RegFlag::HalfCarry, false);
                self.reg.set_flag(RegFlag::Carry, true);
            },
            // NOP
            0x00 => (),
            // Halt CPU
            0x76 => {
                self.state = CpuState::Halted;
            },
            // Stop CPU, maybe other instructions?
            0x10 => {
                match instr.param(0) {
                    // Stop CPU
                    0x00 => {
                        self.state = CpuState::Stopped;
                    },

                    _ => panic!("Instruction not implemented! Opcode {:X} {:X}", instr.opcode(), instr.param(0)),
                }
            },
            // Enable/disable interrupts
            0xF3 => self.intlevel = false,
            0xFB => self.intlevel = true,
            // Left rotate A
            0x07 => {
                let a = self.reg.read(Register::A);
                let rot = self.lrot(a);
                self.reg.write(Register::A, rot);
            },
            // Left rotate A through carry
            0x17 => {
                let a = self.reg.read(Register::A);
                let rot = self.lrot_through(a);
                self.reg.write(Register::A, rot);
            },
            // Right rotate A
            0x0F => {
                let a = self.reg.read(Register::A);
                let rot = self.rrot(a);
                self.reg.write(Register::A, rot);
            },
            // Right rotate A through carry
            0x1F => {
                let a = self.reg.read(Register::A);
                let rot = self.rrot_through(a);
                self.reg.write(Register::A, rot);
            },

            _ => panic!("Instruction not implemented! Opcode {:X}", instr.opcode()),
        }
        let cycles = instr.cycles();
        self.clock += cycles as u64;
        cycles
    }

    pub fn add(&mut self, reg: Register, n: u8) {
        self.add_with_carry(reg, n, false);
    }

    pub fn add_no_writeback(&mut self, x: u8, n: u8, carry_flag: bool) -> u8 {
        let carry = if carry_flag { 1 } else { 0 };
        let halfsum = (x & 0x0F) + (n & 0x0F) + carry;
        let sum = x as u16 + n as u16 + carry as u16;
        let sum_u8 = (sum & 0xFF) as u8;
        self.reg.set_flag(RegFlag::Zero, sum_u8 == 0);
        self.reg.set_flag(RegFlag::Subtract, false);
        self.reg.set_flag(RegFlag::HalfCarry, halfsum > 0x0F);
        self.reg.set_flag(RegFlag::Carry, sum > 0xFF);
        sum_u8
    }

    pub fn add_with_carry(&mut self, reg: Register, n: u8, carry_flag: bool) {
        let x = self.reg.read(reg);
        let sum = self.add_no_writeback(x, n, carry_flag);
        self.reg.write(reg, sum);
    }

    pub fn add_u16(&mut self, reg: Register, n: u16) {
        let x = self.reg.read_u16(reg);
        let halfsum = (x & 0xFFF) as u32 + (n & 0xFFF) as u32; // "Half" = 11th bit
        let sum = x as u32 + n as u32;
        let sum_u16 = (sum & 0xFFFF) as u16;
        self.reg.set_flag(RegFlag::Zero, sum_u16 == 0); // Reference says ignore?
        self.reg.set_flag(RegFlag::Subtract, false);
        self.reg.set_flag(RegFlag::HalfCarry, halfsum > 0xFFF);
        self.reg.set_flag(RegFlag::Carry, sum > 0xFFFF);
        self.reg.write_u16(reg, sum_u16);
    }

    pub fn dec_u16(&mut self, reg: Register) {
        let x = self.reg.read_u16(reg);
        // Behavior at 0 unclear. Do we wrap, or ignore?
        let diff = if x == 0 { 0 } else { x - 1 };
        // Reference says don't set flags?
        self.reg.write_u16(reg, diff);
    }

    pub fn sub(&mut self, reg: Register, n: u8) {
        self.sub_with_carry(reg, n, false);
    }

    pub fn sub_no_writeback(&mut self, x: u8, n: u8, carry_flag: bool) -> u8 {
        let carry = if carry_flag { 1 } else { 0 };
        let xw = Wrapping(x as u16 + carry);
        let Wrapping(halfdiff) = (xw & Wrapping(0x0F)) - Wrapping(n as u16 & 0x0F);
        let Wrapping(diff) = xw - Wrapping(n as u16);
        let diff_u8 = (diff & 0xFF) as u8;
        self.reg.set_flag(RegFlag::Zero, diff_u8 == 0);
        self.reg.set_flag(RegFlag::Subtract, true);
        self.reg.set_flag(RegFlag::HalfCarry, halfdiff > 0x0F);
        self.reg.set_flag(RegFlag::Carry, diff > 0xFF);
        diff_u8
    }

    pub fn sub_with_carry(&mut self, reg: Register, n: u8, carry_flag: bool) {
        let x = self.reg.read(reg);
        let diff = self.sub_no_writeback(x, n, carry_flag);
        self.reg.write(reg, diff);
    }

    pub fn set_bitand_flags(&mut self, x: u8) {
        self.reg.set_flag(RegFlag::Zero, x == 0);
        self.reg.set_flag(RegFlag::Subtract, false);
        self.reg.set_flag(RegFlag::HalfCarry, true);
        self.reg.set_flag(RegFlag::Carry, false);
    }

    pub fn set_bitor_flags(&mut self, x: u8) {
        self.reg.set_flag(RegFlag::Zero, x == 0);
        self.reg.set_flag(RegFlag::Subtract, false);
        self.reg.set_flag(RegFlag::HalfCarry, false);
        self.reg.set_flag(RegFlag::Carry, false);
    }

    pub fn swap_bits_no_writeback(&mut self, n: u8) -> u8 {
        let x = ((n & 0x0F) << 4) | ((n & 0xF0) >> 4);
        self.reg.set_flag(RegFlag::Zero, x == 0);
        self.reg.set_flag(RegFlag::Subtract, false);
        self.reg.set_flag(RegFlag::HalfCarry, false);
        self.reg.set_flag(RegFlag::Carry, false);
        x
    }

    pub fn swap_bits(&mut self, reg: Register) {
        let n = self.reg.read(reg);
        let x = self.swap_bits_no_writeback(n);
        self.reg.write(reg, x);
    }

    /// Left rotate, leaving the old most significant bit in the carry
    pub fn lrot(&mut self, x: u8) -> u8 {
        let shift = (x as u32) << 1;
        let msb = x as u32 & 0x100;
        let rot = ((shift | (msb >> 8)) & 0xFF) as u8;
        self.reg.set_flag(RegFlag::Zero, rot == 0);
        self.reg.set_flag(RegFlag::Subtract, false);
        self.reg.set_flag(RegFlag::HalfCarry, false);
        self.reg.set_flag(RegFlag::Carry, msb != 0);
        rot
    }

    /// Left rotate, treating the carry as part of the value
    pub fn lrot_through(&mut self, x: u8) -> u8 {
        let carry = if self.reg.get_flag(RegFlag::Carry) { 0x100 } else { 0 };
        let shift = (x as u32 | carry) << 1;
        let carry = shift & 0x100;
        let msb = shift & 0x200;
        let rot = ((shift | (msb >> 9)) & 0xFF) as u8;
        self.reg.set_flag(RegFlag::Zero, rot == 0);
        self.reg.set_flag(RegFlag::Subtract, false);
        self.reg.set_flag(RegFlag::HalfCarry, false);
        self.reg.set_flag(RegFlag::Carry, carry != 0);
        rot
    }

    /// Right rotate, leaving the old most significant bit in the carry
    pub fn rrot(&mut self, x: u8) -> u8 {
        let lsb = x & 0x01;
        let shift = x >> 1;
        let rot = shift | (lsb << 7);
        self.reg.set_flag(RegFlag::Zero, rot == 0);
        self.reg.set_flag(RegFlag::Subtract, false);
        self.reg.set_flag(RegFlag::HalfCarry, false);
        self.reg.set_flag(RegFlag::Carry, lsb != 0);
        rot
    }

    /// Right rotate, treating the carry as part of the value
    pub fn rrot_through(&mut self, x: u8) -> u8 {
        let carry = if self.reg.get_flag(RegFlag::Carry) { 0x100 } else { 0 };
        let lsb = x & 0x01;
        let shift = ((x as u32) | carry) >> 1;
        let rot = (shift & 0xFF) as u8;
        self.reg.set_flag(RegFlag::Zero, rot == 0);
        self.reg.set_flag(RegFlag::Subtract, false);
        self.reg.set_flag(RegFlag::HalfCarry, false);
        self.reg.set_flag(RegFlag::Carry, lsb != 0);
        rot
    }

    pub fn lshift(&mut self, x: u8) -> u8 {
        let shift = (x as u32) << 1;
        let carry = shift & 0x100;
        let shift = (shift & 0xFF) as u8;
        self.reg.set_flag(RegFlag::Zero, shift == 0);
        self.reg.set_flag(RegFlag::Subtract, false);
        self.reg.set_flag(RegFlag::HalfCarry, false);
        self.reg.set_flag(RegFlag::Carry, carry != 0);
        shift
    }

    pub fn rshift_logical(&mut self, x: u8) -> u8 {
        let carry = x & 0x01;
        let shift = x >> 1;
        self.reg.set_flag(RegFlag::Zero, shift == 0);
        self.reg.set_flag(RegFlag::Subtract, false);
        self.reg.set_flag(RegFlag::HalfCarry, false);
        self.reg.set_flag(RegFlag::Carry, carry != 0);
        shift
    }

    pub fn rshift_arithmetic(&mut self, x: u8) -> u8 {
        let msb = x & 0x80;
        let lsb = x & 0x01;
        let shift = (x >> 1) | msb;
        self.reg.set_flag(RegFlag::Zero, shift == 0);
        self.reg.set_flag(RegFlag::Subtract, false);
        self.reg.set_flag(RegFlag::HalfCarry, false);
        self.reg.set_flag(RegFlag::Carry, lsb != 0);
        shift
    }
}
