use winit::event_loop::EventLoopProxy;

use crate::app::AppEvent;
use crate::cpu::Cpu;
use crate::debug::Debugger;

pub struct Emulator {
    pub cpu: Cpu,
    pub debugger: Debugger,
}

fn rainbow_rgb(i: f32) -> (u8, u8, u8) {
    let frequency = 0.3;
    let red   = (i * frequency).sin() * 127.0 + 128.0;
    let green = (i * frequency + 2.0).sin() * 127.0 + 128.0;
    let blue  = (i * frequency + 4.0).sin() * 127.0 + 128.0;

    (red as u8, green as u8, blue as u8)
}

impl Emulator {
    /// Create a new emulator instance
    pub fn new() -> Self {
        Emulator {
            cpu: Cpu::new(),
            debugger: Debugger::new(),
        }
    }

    pub fn run_threaded(mut emulator: Emulator, event_loop_proxy: EventLoopProxy<AppEvent>) {
        let mut i = 0.0;
        
        loop {
            // run for approximately 1/60th of a second
            if emulator.run_for_cycles(677_376) {
                // shutdown requested by debugger prompt
                break;
            }

            // Get VRAM frame data (stub: all white)
            let mut frame_data: Vec<u8> = vec![0; 1024 * 512 * 4];
            let (r, g, b) = rainbow_rgb(i);
            for (j, pixel) in frame_data.chunks_exact_mut(4).enumerate() {
                pixel[0] = r;
                pixel[1] = g;
                pixel[2] = b;
                pixel[3] = 255; // Alpha channel
            }

            i += 0.01;

            // Send the frame data to the UI thread
            if event_loop_proxy.send_event(AppEvent::FrameReady(frame_data)).is_err() {
                break;
            }
        }

        let _ = event_loop_proxy.send_event(AppEvent::EmulatorShutdown);
    }

    /// Run the emulator
    pub fn run(&mut self) {
        loop {
            if self.step() {
                // Exit if the debugger has requested to quit
                break;
            }
        }
    }

    /// Run the emulator for a specified number of cycles.
    pub fn run_for_cycles(&mut self, cycles: u64) -> bool {
        for _ in 0..cycles {
            if self.step() {
                // Exit if the debugger has requested to quit
                return true;
            }
        }

        false
    }

    // Perform one step of the emulator cycle.
    pub fn step(&mut self) -> bool {
        if self.debugger.stepping || self.debugger.has_breakpoint(self.cpu.pc) {
            self.debugger.stepping = true;

            if self.debugger.enter(&mut self.cpu) {
                // If the debugger returns true, it means we should quit
                println!("Quitting...");
                return true;
            }
        }

        // Detect the putchar system call and print the character to the
        // console
        if self.cpu.pc == 0xb0 && self.cpu.registers[9] == 0x3d {
            print!("{}", self.cpu.registers[4] as u8 as char);
        }

        self.cpu
            .cop0
            .set_hardware_interrupt(self.cpu.bus.interrupts.should_interrupt());
        self.cpu.step();
        false
    }
}
