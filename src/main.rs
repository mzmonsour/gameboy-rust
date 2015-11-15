use instr::Instr;
use cpu::Cpu;
use cpu::CpuInterrupt;
use time::precise_time_ns;
use std::fs::File;
use std::io::Read;
use std::ops::Index;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

use glium::DisplayBuild;
use glium::Surface;
use glium::SwapBuffersError;
use glium::glutin::Api;
use glium::glutin::GlRequest;
use glium::glutin::Event;

extern crate time;
extern crate getopts;
#[macro_use]
extern crate glium;
extern crate nalgebra;

mod instr;
mod cpu;
mod render;

#[derive(Copy, Clone)]
pub enum MemSection {
    Vram,
    RomBank0,
}

pub struct AddressSpace {
    data: [u8; 0x10000],
}

impl AddressSpace {

    pub fn new() -> AddressSpace {
        AddressSpace { data: [0; 0x10000] }
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

#[derive(Copy, Clone)]
pub enum IntType {
    VblankStart,
    VblankEnd,
}

struct ClockInt {
    pub int_target: u64,
    pub int_type: IntType,
}

impl PartialEq for ClockInt {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.int_target == other.int_target
    }
}

impl Eq for ClockInt {}

impl PartialOrd for ClockInt {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.int_target.partial_cmp(&other.int_target) {
            Some(Ordering::Less) => Some(Ordering::Greater),
            Some(Ordering::Greater) => Some(Ordering::Less),
            ord => ord,
        }
    }
}

impl Ord for ClockInt {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        match self.int_target.cmp(&other.int_target) {
            Ordering::Less => Ordering::Greater,
            Ordering::Greater => Ordering::Less,
            ord => ord,
        }
    }
}

const NS_PER_S: u64 = 1_000_000_000;
const MS_PER_NS: u64 = 1_000_000;

// 10ms
const BUSY_WAIT_THRESHOLD: u64 = 10_000_000;

pub struct Clock {
    freq:       u32,
    period:     u64,
    int_heap:   BinaryHeap<ClockInt>,
}

impl Clock {

    pub fn new(freq: u32) -> Clock {
        Clock {
            freq: freq,
            period: NS_PER_S / (freq as u64),
            int_heap: BinaryHeap::new(),
        }
    }

    pub fn set_interrupt(&mut self, itype: IntType, period: u64) {
        let start = precise_time_ns();
        let int = ClockInt {
            int_type: itype,
            int_target: start + period,
        };
        self.int_heap.push(int);
    }

    pub fn wait_cycles(&mut self, n: u32) -> Option<IntType> {
        let start = precise_time_ns();
        let real_wait = self.period * (n as u64);
        let real_target = real_wait + start;
        let (target, result) = if let Some(interrupt) = self.int_heap.pop() {
            if real_target > interrupt.int_target {
                (interrupt.int_target, Some(interrupt.int_type))
            } else {
                self.int_heap.push(interrupt);
                (real_target, None)
            }
        } else {
            (real_target, None)
        };
        let mut curtime = start;
        if target > start && target - start > BUSY_WAIT_THRESHOLD {
            std::thread::sleep_ms(((target - start) * MS_PER_NS) as u32);
            return result;
        } else {
            loop {
                if curtime >= target {
                    return result;
                }
                curtime = precise_time_ns();
            }
        }
    }

}

fn main() {
    //  Gather command line args
    let args: Vec<String> = std::env::args().collect();
    let mut opts = getopts::Options::new();
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m },
        Err(e) => panic!("Error: {}", e),
    };
    let input = if !matches.free.is_empty() {
        matches.free[0].clone()
    } else {
        println!("No input ROM");
        return;
    };

    // Build graphics context and window
    let display = glium::glutin::WindowBuilder::new()
        .with_title("Gameboy Rust".to_string())
        .with_gl(GlRequest::Specific(Api::OpenGl, (3, 2)))
        .build_glium()
        .unwrap();

    // Do machine initialization
    let mut cpu = Cpu::new();
    {
        let mut ram = cpu.get_ram();
        let mut romfile = match File::open(std::path::Path::new(&input)) {
            Ok(f) => { f },
            Err(e) => {
                println!("Error opening file: {}", e);
                return;
            }
        };
        if let Err(e) = ram.load_rom(&mut romfile) {
            println!("Error loading rom data: {}", e);
            return;
        }
    }

    // Initialize virtual LCD
    let mut lcd = render::GbDisplay::new(&display);

    let mut viewport = {
        let window = display.get_window();
        let (width, height) = window.unwrap().get_inner_size_pixels().unwrap();
        render::calculate_viewport(width, height)
    };

    // Initialize virtual hardware clocks
    let mut clock = Clock::new(cpu::GB_FREQUENCY);
    clock.set_interrupt(IntType::VblankStart, render::VBLANK_PERIOD);

    // Simulate CPU
    'main: loop {
        // Collect user input
        for ev in display.poll_events() {
            match ev {
                Event::Closed => {
                    break 'main;
                },
                Event::Resized(..) => {
                    let window = display.get_window();
                    let (width, height) = window.unwrap().get_inner_size_pixels().unwrap();
                    viewport = render::calculate_viewport(width, height);
                },
                _ => (),
            }
        }

        // Simulate CPU and hardware timers
        'sim: loop  {
            if let Some(int) = clock.wait_cycles(cpu.do_instr()) {
                // Handle timer interrupt
                match int {
                    // Interrupt at the start of the vblank period
                    IntType::VblankStart => {
                        clock.set_interrupt(IntType::VblankStart, render::VBLANK_PERIOD);
                        clock.set_interrupt(IntType::VblankEnd, render::VBLANK_DURATION);
                        cpu.interrupt(CpuInterrupt::Vblank);
                    }
                    // At the end, collect data from VRAM and render it
                    IntType::VblankEnd => {
                        break 'sim;
                    }
                }
            }
        }

        // Redraw screen
        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 0.0, 0.0);
        lcd.clear_viewport(&mut target, viewport, (1.0, 1.0, 1.0, 0.0));
        lcd.draw(&display, &mut target, viewport, cpu.get_ram());
        match target.finish().err() {
            Some(SwapBuffersError::ContextLost) => {
                panic!("OpenGL contetxt lost!");
            },
            Some(SwapBuffersError::AlreadySwapped) => {
                println!("Warning: OpenGL buffer already swapped");
            },
            None => (),
        }
    }
}
