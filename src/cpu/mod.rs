mod arith;
#[cfg(test)]
mod test_utils;

use crate::ram::{AccessSize, Ram};

const NUM_REGISTERS: usize = 32;

pub struct Cpu {
    /// The CPU's general-purpose registers.
    pub registers: [u32; NUM_REGISTERS],

    /// The program counter, which points to the next instruction
    pub pc: u32,

    /// RAM instance to access memory.
    pub ram: Ram,
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
        self.read_memory(address, AccessSize::Word).unwrap()
    }

    /// Execute an instruction
    fn execute(&mut self, instruction: u32) {
        let opcode = instruction >> 26;

        match opcode {
            0x09 => self.ins_addiu(instruction),
            _ => panic!("Unimplemented opcode: {:x} @ {:08x}", opcode, self.pc - 4),
        }
    }

    /// Read a value from memory.
    pub fn read_memory(&self, address: u32, size: AccessSize) -> Result<u32, ()> {
        Ok(self.ram.read(address, size))
    }

    /// Write a value to memory.
    pub fn write_memory(&mut self, address: u32, value: u32, size: AccessSize) -> Result<(), ()> {
        self.ram.write(address, value, size);
        Ok(())
    }
}
