use instr::Instr;
use cpu::Cpu;
use time::precise_time_ns;
use std::fs::File;
use std::io::Read;

extern crate time;

mod instr;
mod cpu;

pub struct AddressSpace {
    data: [u8; 0xFFFF],
}

impl AddressSpace {

    pub fn new() -> AddressSpace {
        AddressSpace { data: [0; 0xFFFF] }
    }

    pub fn read(&self, addr: u16) -> u8 {
        self.data[addr as usize]
    }

    pub fn read_u16(&self, addr: u16) -> u16 {
        self.data[addr as usize] as u16 | ((self.data[addr as usize + 1] as u16) << 8)
    }

    pub fn read_slice(&self, addr: u16, bytes: u16) -> &[u8] {
        &self.data[addr as usize .. (addr + bytes) as usize]
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        self.data[addr as usize] = data;
    }

    pub fn write_u16(&mut self, addr: u16, data: u16) {
        let lo = (data & 0xFF) as u8;
        let hi = ((data & 0xFF00) >> 8) as u8;
        self.data[addr as usize] = lo;
        self.data[addr as usize + 1] = hi;
    }

    pub fn load_rom(&mut self, rom: &mut File) -> std::io::Result<()> {
        // Read in header first
        try!(rom.read(&mut self.data[0x000..0x150]));
        // Then read in remaining cart data
        try!(rom.read(&mut self.data[0x0150..0x8000]));
        Ok(())
    }

}

#[derive(Copy, Clone)]
pub enum RegFlag {
    Zero,
    Subtract,
    HalfCarry,
    Carry,
}

#[derive(Copy, Clone)]
pub enum Register {
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

pub struct RegData {
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

    pub fn new() -> RegData {
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

    pub fn read(&self, reg: Register) -> u8 {
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

    pub fn read_u16(&self, reg: Register) -> u16 {
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

    pub fn write(&mut self, reg: Register, data: u8) {
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

    pub fn write_u16(&mut self, reg: Register, data: u16) {
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

    pub fn copy(&mut self, dst: Register, src: Register) {
        let data = self.read(src);
        self.write(dst, data);
    }

    pub fn copy_u16(&mut self, dst: Register, src: Register) {
        let data = self.read_u16(src);
        self.write_u16(dst, data);
    }

    pub fn set_flag(&mut self, flag: RegFlag, on: bool) {
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

    pub fn get_flag(&self, flag: RegFlag) -> bool {
        let bit = match flag {
            RegFlag::Zero => 0x80,
            RegFlag::Subtract => 0x40,
            RegFlag::HalfCarry => 0x20,
            RegFlag::Carry => 0x10,
        };
        self.flag & bit != 0
    }

    pub fn advance_pc(&mut self) -> u16 {
        let pc = self.pc;
        self.pc += 1;
        pc
    }

    pub fn set_pc(&mut self, addr: u16) -> u16 {
        let pc = self.pc;
        self.pc = addr;
        pc
    }

    pub fn add_pc(&mut self, n: i8) -> u16 {
        let pc = self.pc;
        self.pc = ((self.pc as i32) + (n as i32)) as u16;
        pc
    }

}

fn main() {
    let cpu = Cpu::new();
    'main: loop {
        println!("Nothing to do, breaking out of main");
        break 'main;
    }
}
