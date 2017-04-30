use instr::Instr;
use cpu::Cpu;
use cpu::CpuInterrupt;
use time::precise_time_ns;
use std::fs::File;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::thread;
use std::sync::mpsc;
use std::sync::mpsc::TryRecvError;

use glium::DisplayBuild;
use glium::Surface;
use glium::SwapBuffersError;
use glium::glutin::Api;
use glium::glutin::GlRequest;
use glium::glutin::Event;

use mem::{RwMemory, WriteObserver};

extern crate time;
extern crate getopts;
#[macro_use]
extern crate glium;
extern crate cgmath;

mod instr;
mod cpu;
mod mem;
mod render;

#[derive(Copy, Clone)]
pub enum IntType {
    Vblank,
    Hblank,
    IoTimer,
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
const NS_PER_MS: u64 = 1_000_000;

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
            std::thread::sleep_ms(((target - start) / NS_PER_MS) as u32);
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

pub enum WorkerCmd {
    TakeSnapshot(Box<RwMemory>, WriteObserver),
    Shutdown,
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
    cpu.init();
    {
        let mut ram = cpu.get_ram();
        let mut biosfile = match File::open(std::path::Path::new("rom/bios.bin")) {
            Ok(f) => { f },
            Err(e) => {
                println!("Error opening bios file");
                return;
            },
        };
        let mut romfile = match File::open(std::path::Path::new(&input)) {
            Ok(f) => { f },
            Err(e) => {
                println!("Error opening file: {}", e);
                return;
            }
        };
        if let Err(e) = ram.load_bios(&mut biosfile) {
            println!("Error loading bios data: {}", e);
            return;
        }
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

    let (io_tx, sim_rx) = mpsc::channel();
    let (sim_tx, io_rx) = mpsc::channel();
    let sim_worker = thread::Builder::new()
        .name("simulation worker".to_string())
        .spawn(move || {

        // Initialize virtual hardware clocks
        let mut clock = Clock::new(cpu::GB_FREQUENCY);
        clock.set_interrupt(IntType::Vblank, render::VBLANK_PERIOD);
        clock.set_interrupt(IntType::Hblank, render::HBLANK_PERIOD);
        clock.set_interrupt(IntType::IoTimer, cpu::TIMER_BASE_PERIOD_NS);

        // TODO: Abstract LCD simulation better
        // Track ly here
        let mut ly = 0;

        'main: loop {
            // Simulate CPU and hardware timers
            'sim: loop  {
                if let Some(int) = clock.wait_cycles(cpu.do_instr()) {
                    // Handle timer interrupt
                    match int {
                        // Interrupt at the start of the vblank period
                        IntType::Vblank => {
                            clock.set_interrupt(IntType::Vblank, render::VBLANK_PERIOD);
                            cpu.interrupt(CpuInterrupt::Vblank);
                            ly = 144; // set_ly_vblank
                            let ram = cpu.get_ram();
                            ram.sys_write(mem::IOREG_LY, ly);
                        }
                        // ~10 H-Blanks occur after the V-Blank starts
                        IntType::Hblank => {
                            clock.set_interrupt(IntType::Hblank, render::HBLANK_PERIOD);
                            // inc_ly_counter
                            if ly >= 153 {
                                ly = 0;
                            } else {
                                ly += 1
                            }
                            let ram = cpu.get_ram();
                            ram.sys_write(mem::IOREG_LY, ly);
                            // At the end, collect data from VRAM and render it
                            if ly == 0 {
                                break 'sim;
                            }
                        }
                        // Do timer computations
                        IntType::IoTimer => {
                            clock.set_interrupt(IntType::IoTimer, cpu::TIMER_BASE_PERIOD_NS);
                            cpu.inc_io_timer();
                        }
                    }
                }
            }

            // Check commands from master
            match sim_rx.try_recv() {
                Ok(WorkerCmd::TakeSnapshot(oldsnap, mut observer)) => {
                    let ram = cpu.get_ram();
                    let newsnap = ram.swap_backup(oldsnap);
                    ram.get_observer().apply(&mut observer);
                    sim_tx.send((newsnap, observer));
                    if !ram.verify_backup() {
                        println!("Backup verify failed!")
                    }
                },
                Ok(WorkerCmd::Shutdown) => break 'main,
                Err(TryRecvError::Empty) => (),
                Err(TryRecvError::Disconnected) => {
                    panic!("I/O thread disconnected without notifying");
                }
            }
        }
    });

    // Create a memory snapshot, and write observer
    let mut oldsnap = Some(Box::new(RwMemory::new()));
    let mut oldobserver = Some(WriteObserver::new());

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

        // Request memory snapshot from simulation
        io_tx.send(WorkerCmd::TakeSnapshot(oldsnap.take().unwrap(), oldobserver.take().unwrap()));
        let (snapshot, mut observer) = match io_rx.recv() {
            Ok(v) => v,
            Err(_) => panic!("Did not receive snapshot from simulation thread"),
        };

        // Redraw screen
        let pre_clear = precise_time_ns();
        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 0.0, 1.0);
        lcd.clear_viewport(&mut target, viewport, (1.0, 1.0, 1.0, 1.0));
        let post_clear = precise_time_ns();
        let clear_time = (post_clear - pre_clear) as f32 / NS_PER_MS as f32;
        if clear_time > 5.0f32 {
            println!("clear time: {}ms", clear_time);
        }
        let pre_draw = precise_time_ns();
        lcd.draw(&display, &mut target, viewport, &snapshot, &mut observer);
        let post_draw = precise_time_ns();
        let draw_time = (post_draw - pre_draw) as f32 / NS_PER_MS as f32;
        if draw_time > 5.0f32 {
            println!("lcd.draw time: {}ms", draw_time);
        }
        match target.finish().err() {
            Some(SwapBuffersError::ContextLost) => {
                panic!("OpenGL contetxt lost!");
            },
            Some(SwapBuffersError::AlreadySwapped) => {
                println!("Warning: OpenGL buffer already swapped");
            },
            None => (),
        }
        let pre_flush = precise_time_ns();
        display.flush();
        let post_flush = precise_time_ns();
        let flush_time = (post_flush - pre_flush) as f32 / NS_PER_MS as f32;
        if flush_time > 5.0f32 {
            println!("flush time: {}ms", flush_time);
        }

        oldsnap = Some(snapshot);
        oldobserver = Some(observer);
    }

    // Shutdown sim thread
    io_tx.send(WorkerCmd::Shutdown);
}
