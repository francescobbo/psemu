mod arith;

use crate::ram::Ram;

const NUM_REGISTERS: usize = 32;

#[derive(Debug)]
pub struct Cpu {
    /// RAM instance to access memory.
    pub ram: Ram,

    /// The CPU's general-purpose registers.
    pub registers: [u32; NUM_REGISTERS],

    /// The program counter, which points to the next instruction
    pub pc: u32,
}

impl Cpu {
    pub fn new() -> Self {
        Cpu {
            ram: Ram::new(),
            registers: [0; NUM_REGISTERS],
            pc: 0,
        }
    }

    /// Perform one step of the CPU cycle.
    pub fn step(&mut self) {
        // Fetch the instruction at the current program counter (PC).
        let instruction = self.fetch_instruction(self.pc);

        // Update the program counter to point to the next instruction.
        self.pc += 4;

        // Execute the instruction based on its opcode and function code.
        self.execute(instruction);
    }

    /// Fetch the instruction from the given address.
    fn fetch_instruction(&self, address: u32) -> u32 {
        self.ram.read32(address)
    }

    /// Execute an instruction
    fn execute(&mut self, instruction: u32) {
        let opcode = instruction >> 26;

        match opcode {
            0x09 => self.ins_addiu(instruction),
            _ => panic!("Unimplemented opcode: {:#X}", opcode),
        }
    }
}
