//[ mod-arith
mod arith;
//] mod-arith
//[ mod-instruction
mod instruction;
//[ !omit
//[ mod-test-utils
#[cfg(test)]
mod test_utils;
//] mod-test-utils
//] !omit

//[ !omit
use crate::ram::Ram;
//] !omit
use instruction::Instruction;
//] mod-instruction

pub enum AccessSize {
    Byte,
    HalfWord,
    Word,
}

const NUM_REGISTERS: usize = 32;

/// The emulated PS1 CPU
pub struct Cpu {
    pub registers: [u32; NUM_REGISTERS],
    pub pc: u32,

    pub ram: Ram,
}

impl Cpu {
    pub fn new() -> Self {
        Cpu {
            registers: [0; NUM_REGISTERS],
            pc: 0,
            ram: Ram::new(),
        }
    }

    //[ cpu-stubs
    pub fn step(&mut self) {
        // Fetch the instruction at the current program counter (PC).
        let instruction = self.fetch_instruction(self.pc).unwrap();

        // Update the program counter to point to the next instruction.
        self.pc += 4;

        // Execute the instruction based on its opcode and function code.
        self.execute(instruction);
    }

    //[ fetch-execute
    fn fetch_instruction(&self, address: u32) -> Result<Instruction, ()> {
        let value = self.read_memory(address, AccessSize::Word)?;
        Ok(Instruction(value))
    }

    fn execute(&mut self, instruction: Instruction) {
        let opcode = instruction.opcode();
        match opcode {
            0x09 => self.ins_addiu(instruction),
            //] fetch-execute
            _ => {
                // For any other opcode, we'll panic for now.
                // Later, this will cause an "Illegal Instruction" exception.
                panic!(
                    "Unimplemented opcode: {opcode:02x} @ {:08x}",
                    self.pc - 4
                );
            }
        }
    }

    pub fn read_memory(
        &self,
        address: u32,
        size: AccessSize,
    ) -> Result<u32, ()> {
        Ok(self.ram.read(address, size))
    }

    pub fn write_memory(
        &mut self,
        address: u32,
        value: u32,
        size: AccessSize,
    ) -> Result<(), ()> {
        self.ram.write(address, value, size);
        Ok(())
    }
    //] cpu-stubs
    //[ ins-helpers
    /// Get the value of the GPR register pointed to by rs
    fn get_rs(&self, instruction: Instruction) -> u32 {
        self.registers[instruction.rs()]
    }

    /// Write a value to a GPR register
    fn write_reg(&mut self, index: usize, value: u32) {
        // The zero register (R0) is always 0, so we don't allow writing to it
        if index != 0 {
            self.registers[index] = value;
        }
    }
    //] ins-helpers
}
//] cpu-new
