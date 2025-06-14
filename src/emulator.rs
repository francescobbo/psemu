use std::sync::atomic::Ordering;

use winit::event_loop::EventLoopProxy;

use crate::app::AppEvent;
use crate::cpu::Cpu;
use crate::debugger::Debugger;

pub struct Emulator {
    pub cpu: Cpu,
    pub debugger: Debugger,

    pub cycles: u64,
}

impl Emulator {
    /// Create a new emulator instance
    pub fn new() -> Self {
        let cpu = Cpu::new();
        let debugger = Debugger::new();
        let breakpoint = debugger.triggered.clone();
        ctrlc::set_handler(move || {
            breakpoint.store(true, Ordering::SeqCst);
        })
        .expect("Error setting Ctrl-C handler");

        Emulator {
            cpu,
            debugger,
            cycles: 0,
        }
    }

    pub fn run_threaded(
        mut emulator: Emulator,
        event_loop_proxy: EventLoopProxy<AppEvent>,
    ) {
        loop {
            // run for approximately 1/60th of a second
            if emulator.run_for_cycles(677_376) {
                // shutdown requested by debugger prompt
                break;
            }

            // Get VRAM frame data (stub: all white)
            let mut frame_data: Vec<u8> = vec![0; 1024 * 512 * 4];
            for (j, pixel) in frame_data.chunks_exact_mut(4).enumerate() {
                let (r, g, b) = emulator.cpu.bus.gpu.get_pixel_color(j);

                pixel[0] = r;
                pixel[1] = g;
                pixel[2] = b;
                pixel[3] = 255; // Alpha channel
            }

            // Send the frame data to the UI thread
            if event_loop_proxy
                .send_event(AppEvent::FrameReady(frame_data))
                .is_err()
            {
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

    pub fn run_until(&mut self, pc: u32) -> bool {
        while self.cpu.pc != pc {
            if self.step() {
                // Exit if the debugger has requested to quit
                return true;
            }
        }
        false
    }

    // Perform one step of the emulator cycle.
    pub fn step(&mut self) -> bool {
        let break_reason = self.debugger.break_reason(&self.cpu);
        if let Some(reason) = break_reason {
            if self.debugger.enter(&mut self.cpu, reason) {
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

        self.cycles += 1;

        false
    }
}
