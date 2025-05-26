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

    /// Run the emulator
    pub fn run(&mut self) {
        loop {
            if self.debugger.quit {
                println!("Quitting...");
                break;
            } else if self.debugger.stepping || self.debugger.has_breakpoint(self.cpu.pc) {
                self.debugger.stepping = true;
                self.debugger.enter(&mut self.cpu);
            }

            self.cpu.step();
        }
    }
}
