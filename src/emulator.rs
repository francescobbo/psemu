use crate::cpu::Cpu;
use crate::debug::Debugger;

pub struct Emulator {
    pub cpu: Cpu,
    pub debugger: Debugger,
}

impl Emulator {
    /// Create a new emulator instance
    pub fn new() -> Self {
        Emulator {
            cpu: Cpu::new(),
            debugger: Debugger::new(),
        }
    }

    pub fn run_threaded(mut emulator: Emulator) {
        loop {
            // run for approximately 1/60th of a second
            if emulator.run_for_cycles(677_376) {
                // shutdown requested by debugger prompt
                break;
            }

            // Get VRAM frame data (stub: all white)
            let frame_data: Vec<u8> = vec![0xff; 1024 * 512 * 4];

            // Send the frame data to the UI thread
            // if proxy.send_event(AppEvent::FrameReady(frame_data)).is_err() {
            //     break;
            // }
        }

        // let _ = proxy.send_event(AppEvent::EmulatorShutdown);
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
