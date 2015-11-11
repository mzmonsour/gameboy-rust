struct AddressSpace {
    data: [u8; 0xFFFF],
}

impl AddressSpace {

    fn new() -> AddressSpace {
        AddressSpace { data: [0; 0xFFFF] }
    }

    fn read(&self, addr: u16) -> u8 {
        self.data[addr as usize]
    }

    fn read_u16(&self, addr: u16) -> u16 {
        self.data[addr as usize] as u16 | ((self.data[addr as usize + 1] as u16) << 8)
    }

    fn read_slice(&self, addr: u16, bytes: u16) -> &[u8] {
        &self.data[addr as usize .. (addr + bytes) as usize]
    }

    fn write(&mut self, addr: u16, data: u8) {
        self.data[addr as usize] = data;
    }

    fn write_u16(&mut self, addr: u16, data: u16) {
        let lo = (data & 0xFF) as u8;
        let hi = ((data & 0xFF00) >> 8) as u8;
        self.data[addr as usize] = lo;
        self.data[addr as usize + 1] = hi;
    }

}

struct Instr {
    opcode: u8,
    data: Vec<u8>,
    cycles: u32,
}

impl Instr {

    fn parse(reg: &mut RegData, rom: &AddressSpace) -> Instr {
        let opcode = rom.read(reg.advance_pc());
        Instr {
            opcode: opcode,
            data: Vec::new(),
            cycles: 4,
        }
    }

    fn opcode(&self) -> u8 {
        self.opcode
    }

    fn cycles(&self) -> u32 {
        self.cycles
    }

    fn param(&self, i: usize) -> u8 {
        self.data[i]
    }

    fn param_u16(&self, i: usize) -> u16 {
        (self.data[i] as u16) | ((self.data[i + 1] as u16) << 8)
    }

}

#[derive(Copy, Clone)]
enum RegFlag {
    Zero,
    Subtract,
    HalfCarry,
    Carry,
}

#[derive(Copy, Clone)]
enum Register {
    A,
    B,
    C,
    D,
    E,
    F,
    H,
    L,
    AF,
    BC,
    DE,
    HL,
    SP,
    PC,
    Flag,
}

struct RegData {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    f: u8,
    h: u8,
    l: u8,
    sp: u16,
    pc: u16,
    flag: u8,
}

impl RegData {

    fn new() -> RegData {
        RegData {
            a: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            f: 0,
            h: 0,
            l: 0,
            sp: 0xFFFE,
            pc: 0x100,
            flag: 0,
        }
    }

    fn read(&self, reg: Register) -> u8 {
        match reg {
            Register::A => self.a,
            Register::B => self.b,
            Register::C => self.c,
            Register::D => self.d,
            Register::E => self.e,
            Register::F => self.f,
            Register::H => self.h,
            Register::L => self.l,
            Register::Flag => self.flag,
            _ => panic!("Register not available for 8-bit read"),
        }
    }

    fn read_u16(&self, reg: Register) -> u16 {
        match reg {
            Register::AF => (self.a as u16) << 8 | self.f as u16,
            Register::BC => (self.b as u16) << 8 | self.c as u16,
            Register::DE => (self.d as u16) << 8 | self.e as u16,
            Register::HL => (self.h as u16) << 8 | self.l as u16,
            Register::SP => self.sp,
            Register::PC => self.pc,
            _ => panic!("Register not available for 16-bit read"),
        }
    }

    fn write(&mut self, reg: Register, data: u8) {
        match reg {
            Register::A => self.a = data,
            Register::B => self.b = data,
            Register::C => self.c = data,
            Register::D => self.d = data,
            Register::E => self.e = data,
            Register::F => self.f = data,
            Register::H => self.h = data,
            Register::L => self.l = data,
            _ => panic!("Register not available for 8-bit write"),
        }
    }

    fn write_u16(&mut self, reg: Register, data: u16) {
        let (hi, lo) = ((data & 0xFF00 >> 8) as u8, (data & 0xFF) as u8);
        match reg {
            Register::AF => {
                self.a = hi;
                self.f = lo;
            },
            Register::BC => {
                self.b = hi;
                self.c = lo;
            },
            Register::DE => {
                self.d = hi;
                self.c = lo;
            },
            Register::HL => {
                self.h = hi;
                self.l = lo;
            },
            Register::SP => self.sp = data,
            Register::PC => self.pc = data,
            _ => panic!("Register not available for 16-bit write"),
        }
    }

    fn copy(&mut self, dst: Register, src: Register) {
        let data = self.read(src);
        self.write(dst, data);
    }

    fn copy_u16(&mut self, dst: Register, src: Register) {
        let data = self.read_u16(src);
        self.write_u16(dst, data);
    }

    fn set_flag(&mut self, flag: RegFlag, on: bool) {
        let bit = match flag {
            RegFlag::Zero => 0x80,
            RegFlag::Subtract => 0x40,
            RegFlag::HalfCarry => 0x20,
            RegFlag::Carry => 0x10,
        };
        if (on) {
            self.flag |= bit;
        } else {
            self.flag &= bit ^ 0xFF;
        }
    }

    fn get_flag(&self, flag: RegFlag) -> bool {
        let bit = match flag {
            RegFlag::Zero => 0x80,
            RegFlag::Subtract => 0x40,
            RegFlag::HalfCarry => 0x20,
            RegFlag::Carry => 0x10,
        };
        self.flag & bit != 0
    }

    fn advance_pc(&mut self) -> u16 {
        let pc = self.pc;
        self.pc += 1;
        pc
    }

}

static GB_FREQUENCY: u32 = 4194304;

struct Cpu {
    reg: RegData,
    ram: AddressSpace,
    freq: u32,
    clock: u64,
    cycle_block: u32,
}

impl Cpu {

    fn new() -> Cpu {
        Cpu::new_with_freq(GB_FREQUENCY)
    }

    fn new_with_freq(freq: u32) -> Cpu {
        Cpu {
            reg: RegData::new(),
            ram: AddressSpace::new(),
            freq: freq,
            clock: 0,
            cycle_block: 0,
        }
    }

    fn do_cycle(&mut self) {
        self.clock += 1;
        if self.cycle_block == 0 {
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
                _ => panic!("Instruction not implemented! Opcode {}", instr.opcode()),
            }
            self.cycle_block = instr.cycles();
        }
        self.cycle_block -= 1;
    }

    fn add(&mut self, reg: Register, n: u8) {
        self.add_with_carry(reg, n, false);
    }

    fn add_with_carry(&mut self, reg: Register, n: u8, carry_flag: bool) {
        let carry = if carry_flag { 1 } else { 0 };
        let x = self.reg.read(reg);
        let halfsum = (x & 0x0F) + (n & 0x0F) + carry;
        let sum = x as u16 + n as u16 + carry as u16;
        let sum_u8 = (sum & 0xFF) as u8;
        self.reg.set_flag(RegFlag::Zero, sum_u8 == 0);
        self.reg.set_flag(RegFlag::Subtract, false);
        self.reg.set_flag(RegFlag::HalfCarry, halfsum > 0x0F);
        self.reg.set_flag(RegFlag::Carry, sum > 0xFF);
        self.reg.write(reg, sum_u8);
    }

}

fn main() {
    println!("Hello, world!");
}
