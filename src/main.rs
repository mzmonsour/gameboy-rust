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

    fn read_slice(&self, addr: u16, bytes: u16) -> &[u8] {
        self.data.slice(addr as usize, (addr + bytes) as usize)
    }

    fn write(&mut self, addr: u16, data: u8) {
        self.data[addr as usize] = data;
    }

}

struct Instr<'a> {
    opcode: u8,
    data: Option<&'a [u8]>,
    cycles: usize,
}

impl<'a> Instr<'a> {

    fn parse(reg: &mut RegData, rom: &'a AddressSpace) -> Instr<'a> {
        let opcode = rom.read(reg.advance_pc());
        Instr {
            opcode: opcode,
            data: None,
            cycles: 4,
        }
    }

    fn opcode(&self) -> u8 {
        self.opcode
    }

    fn cycles(&self) -> usize {
        self.cycles
    }

    fn param(&self, i: usize) -> u8 {
        self.data.expect("Instruction type does not carry parameters")[i]
    }

}

enum RegFlag {
    Zero,
    Subtract,
    HalfCarry,
    Carry,
}

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
        if self.cycle_block > 0 {
            self.cycle_block -= 1;
        } else {
            let instr = Instr::parse(&mut self.reg, &self.ram);
            match instr.opcode() {
                _ => panic!("Instruction not implemented!"),
            }
        }
    }

}

fn main() {
    println!("Hello, world!");
}
