use std::fs::File;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::ops::Index;
use std::ops::IndexMut;

#[derive(Copy, Clone)]
pub enum MemSection {
    Vram,
    RomBank0,
}

pub const REGION_TILEDATA_UNSIGNED_BEG: u16 = 0x8000;
pub const REGION_TILEDATA_UNSIGNED_END: u16 = 0x8FFF;
pub const REGION_TILEDATA_SIGNED_BEG:   u16 = 0x8800;
pub const REGION_TILEDATA_SIGNED_END:   u16 = 0x97FF;
pub const REGION_TILEMAP0_BEG:          u16 = 0x9800;
pub const REGION_TILEMAP0_END:          u16 = 0x9BFF;
pub const REGION_TILEMAP1_BEG:          u16 = 0x9C00;
pub const REGION_TILEMAP1_END:          u16 = 0x9FFF;

pub enum Region {
    TileDataUnsigned,
    TileDataSigned,
    TileMap0,
    TileMap1,
}

pub struct WriteObserver {
    td_signed_dirty:    bool,
    td_unsigned_dirty:  bool,
    tmap0_dirty:        bool,
    tmap1_dirty:        bool,
}

impl WriteObserver {

    pub fn new() -> WriteObserver {
        WriteObserver {
            td_signed_dirty:    true,
            td_unsigned_dirty:  true,
            tmap0_dirty:        true,
            tmap1_dirty:        true,
        }
    }

    pub fn record_write(&mut self, addr: u16) {
        match addr {
            REGION_TILEDATA_UNSIGNED_BEG...REGION_TILEDATA_UNSIGNED_END => {
                self.td_unsigned_dirty = true;
            },
            REGION_TILEDATA_SIGNED_BEG...REGION_TILEDATA_SIGNED_END => {
                self.td_signed_dirty = true;
            },
            REGION_TILEMAP0_BEG...REGION_TILEMAP0_END => {
                self.tmap0_dirty = true;
            },
            REGION_TILEMAP1_BEG...REGION_TILEMAP1_END => {
                self.tmap1_dirty = true;
            },
            _ => (),
        }
    }

    pub fn check_dirty(&self, region: Region) -> bool {
        match region {
            Region::TileDataUnsigned => {
                self.td_unsigned_dirty
            },
            Region::TileDataSigned => {
                self.td_signed_dirty
            },
            Region::TileMap0 => {
                self.tmap0_dirty
            },
            Region::TileMap1 => {
                self.tmap1_dirty
            },
        }
    }

    pub fn clean_region(&mut self, region: Region) {
        match region {
            Region::TileDataUnsigned => {
                self.td_unsigned_dirty = false;
            },
            Region::TileDataSigned => {
                self.td_signed_dirty = false;
            },
            Region::TileMap0 => {
                self.tmap0_dirty = false;
            },
            Region::TileMap1 => {
                self.tmap1_dirty = false;
            },
        }
    }

    /// Applies dirtiness to another WriteObserver, then cleans self
    pub fn apply(&mut self, other: &mut WriteObserver) {
        other.td_unsigned_dirty |= self.td_unsigned_dirty;
        other.td_signed_dirty |= self.td_signed_dirty;
        other.tmap0_dirty |= self.tmap0_dirty;
        other.tmap1_dirty |= self.tmap1_dirty;
        self.clean_all();
    }

    pub fn clean_all(&mut self) {
        self.td_unsigned_dirty = false;
        self.td_signed_dirty = false;
        self.tmap0_dirty = false;
        self.tmap1_dirty = false;
    }

}

pub struct RwMemory {
    data: [u8; 0x10000],
}

impl RwMemory {

    fn new() -> RwMemory {
        RwMemory {
            data: [0; 0x10000],
        }
    }

    fn copy_to(&self, other: &mut RwMemory) {
        other.data.copy_from_slice(&self.data);
    }
}

impl Index<u16> for RwMemory {
    type Output = u8;

    fn index(&self, idx: u16) -> &u8 {
        &self.data[idx as usize]
    }
}

impl IndexMut<u16> for RwMemory {
    fn index_mut(&mut self, idx: u16) -> &mut u8 {
        &mut self.data[idx as usize]
    }
}

// TODO: Separate ROM from RwMemory
pub struct AddressSpace {
    bios:           [u8; 0x100],
    main_ram:       RwMemory,
    backup_ram:     Box<RwMemory>,
    bios_readable:  bool,
    observer:       WriteObserver,
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
pub const IOREG_DMA:    u16 = 0xFF46;
pub const IOREG_BGP:    u16 = 0xFF47;
pub const IOREG_OBP0:   u16 = 0xFF48;
pub const IOREG_OBP1:   u16 = 0xFF49;
pub const IOREG_WY:     u16 = 0xFF4A;
pub const IOREG_WX:     u16 = 0xFF4B;
pub const IOREG_BIOSRW: u16 = 0xFF50;
pub const IOREG_IE:     u16 = 0xFFFF;

impl AddressSpace {

    pub fn new() -> AddressSpace {
        AddressSpace {
            bios: [0; 0x100],
            main_ram: RwMemory::new(),
            backup_ram: Box::new(RwMemory::new()),
            bios_readable: true,
            observer: WriteObserver::new(),
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        if addr < 0x100 && self.bios_readable {
            self.bios[addr as usize]
        } else {
            self.main_ram[addr]
        }
    }

    pub fn read_u16(&self, addr: u16) -> u16 {
        let lo = self.read(addr) as u16;
        let hi = (self.read(addr + 1) as u16) << 8;
        lo | hi
    }

    pub fn read_slice(&self, addr: u16, bytes: u16) -> &[u8] {
        &self.main_ram.data[addr as usize .. (addr + bytes) as usize]
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
            // Don't write, but begin a DMA instead
            IOREG_DMA => {
                let to_addr = 0xFE00;
                let from_addr = (data as u16) << 8;
                for i in 0x000..0x100 {
                    self.main_ram[to_addr + i] = self.main_ram[from_addr + i];
                    self.backup_ram[to_addr + i] = self.main_ram[from_addr + i];
                }
                true
            },
            // Disable access to BIOS memory
            IOREG_BIOSRW => {
                if self.bios_readable && data == 1 {
                    self.bios_readable = false;
                }
                false
            },
            // No special write rules
            _ => true,
        };
        if rw {
            self.sys_write(addr, data);
        }
    }

    pub fn write_u16(&mut self, addr: u16, data: u16) {
        let lo = (data & 0xFF) as u8;
        let hi = ((data & 0xFF00) >> 8) as u8;
        self.write(addr, lo);
        self.write(addr + 1, hi);
    }

    /// System write, bypasses read-only flag
    pub fn sys_write(&mut self, addr: u16, data: u8) {
        self.observer.record_write(addr);
        self.main_ram[addr] = data;
        self.backup_ram[addr] = data;
    }

    pub fn load_bios(&mut self, bios: &mut File) -> ::std::io::Result<()> {
        try!(bios.read(&mut self.bios[0x000..0x100]));
        Ok(())
    }

    pub fn load_rom(&mut self, rom: &mut File) -> ::std::io::Result<()> {
        // Read in header first
        try!(rom.read(&mut self.main_ram.data[0x000..0x150]));
        // Then read in remaining cart data
        try!(rom.read(&mut self.main_ram.data[0x0150..0x8000]));
        Ok(())
    }

    pub fn set_bios_readable(&mut self) {
        self.bios_readable = true;
    }

    pub fn get_observer(&mut self) -> &mut WriteObserver {
        &mut self.observer
    }

    pub fn swap_backup(&mut self, mut mem: Box<RwMemory>) -> Box<RwMemory> {
        ::std::mem::swap(&mut mem, &mut self.backup_ram);
        let AddressSpace {
            ref mut main_ram,
            ref mut backup_ram,
            ..
        } = *self;
        main_ram.copy_to(backup_ram);
        mem
    }

}

impl Index<u16> for AddressSpace {
    type Output = u8;

    fn index(&self, idx: u16) -> &u8 {
        &self.main_ram[idx]
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
            pc: 0x000,
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
        let (hi, lo) = (((data & 0xFF00) >> 8) as u8, (data & 0xFF) as u8);
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
                self.e = lo;
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

    pub fn get_pc(&self) -> u16 {
        self.pc
    }

}
