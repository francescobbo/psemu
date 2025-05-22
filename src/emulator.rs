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
            if self.step() {
                // If the step returns true, it means we should quit
                break;
            }
        }
    }

    // Perform one step of the emulator cycle.
    pub fn step(&mut self) -> bool {
        if self.debugger.stepping {
            if self.debugger.enter(&mut self.cpu) {
                // If the debugger returns true, it means we should quit
                return true;
            }
        }

        self.cpu.step();
        false
    }
}
