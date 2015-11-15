use std::fs::File;
use std::io::Read;
use std::ops::Index;

#[derive(Copy, Clone)]
pub enum MemSection {
    Vram,
    RomBank0,
}

pub struct AddressSpace {
    data: [u8; 0x10000],
}

pub const IOREG_P1:     u16 = 0xFF00;
pub const IOREG_SB:     u16 = 0xFF01;
pub const IOREG_DIV:    u16 = 0xFF04;
pub const IOREG_TIMA:   u16 = 0xFF05;
pub const IOREG_TMA:    u16 = 0xFF06;
pub const IOREG_TAC:    u16 = 0xFF07;
pub const IOREG_IF:     u16 = 0xFF0F;
pub const IOREG_LCDC:   u16 = 0xFF40;
pub const IOREG_STAT:   u16 = 0xFF41;
pub const IOREG_SCY:    u16 = 0xFF42;
pub const IOREG_SCX:    u16 = 0xFF43;
pub const IOREG_LY:     u16 = 0xFF44;
pub const IOREG_LYC:    u16 = 0xFF45;
pub const IOREG_BGP:    u16 = 0xFF47;
pub const IOREG_OBP0:   u16 = 0xFF48;
pub const IOREG_OBP1:   u16 = 0xFF49;
pub const IOREG_WY:     u16 = 0xFF4A;
pub const IOREG_WX:     u16 = 0xFF4B;
pub const IOREG_IE:     u16 = 0xFFFF;

impl AddressSpace {

    pub fn new() -> AddressSpace {
        AddressSpace { data: [0; 0x10000] }
    }

    pub fn read(&self, addr: u16) -> u8 {
        self.data[addr as usize]
    }

    pub fn read_u16(&self, addr: u16) -> u16 {
        let lo = self.read(addr) as u16;
        let hi = (self.read(addr + 1) as u16) << 8;
        lo | hi
    }

    pub fn read_slice(&self, addr: u16, bytes: u16) -> &[u8] {
        &self.data[addr as usize .. (addr + bytes) as usize]
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        let mut addr = addr;
        let mut data = data;
        let rw = match addr {
            // ROM Banks, read only
            0x0000...0x7FFF => false,
            // Switchable RAM bank
            // TODO: Make RAM switchable
            0xA000...0xBFFF => true,
            // Internal RAM echo
            0xE000...0xFDFF => {
                addr -= 0x2000;
                true
            },
            // I/O registers
            IOREG_DIV => {
                data = 0;
                true
            },
            IOREG_LY => {
                data = 0;
                true
            },
            // No special write rules
            _ => true,
        };
        if rw {
            self.data[addr as usize] = data;
        }
    }

    pub fn write_u16(&mut self, addr: u16, data: u16) {
        let lo = (data & 0xFF) as u8;
        let hi = ((data & 0xFF00) >> 8) as u8;
        self.write(addr, lo);
        self.write(addr + 1, hi);
    }

    pub fn load_rom(&mut self, rom: &mut File) -> ::std::io::Result<()> {
        // Read in header first
        try!(rom.read(&mut self.data[0x000..0x150]));
        // Then read in remaining cart data
        try!(rom.read(&mut self.data[0x0150..0x8000]));
        Ok(())
    }

}

impl Index<u16> for AddressSpace {
    type Output = u8;

    fn index(&self, idx: u16) -> &u8 {
        &self.data[idx as usize]
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
