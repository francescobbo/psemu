mod arith;
mod instruction;
mod load_store;
mod logic;
#[cfg(test)]
mod test_utils;

use crate::ram::{AccessSize, Ram};
pub use instruction::Instruction;

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
    fn fetch_instruction(&self, address: u32) -> Instruction {
        Instruction(self.read_memory(address, AccessSize::Word).unwrap())
    }

    /// Execute an instruction
    fn execute(&mut self, instruction: Instruction) {
        match instruction.opcode() {
            0x08 => self.ins_addi(instruction),
            0x09 => self.ins_addiu(instruction),
            0x0a => self.ins_slti(instruction),
            0x0b => self.ins_sltiu(instruction),
            0x0c => self.ins_andi(instruction),
            0x0d => self.ins_ori(instruction),
            0x0e => self.ins_xori(instruction),
            0x0f => self.ins_lui(instruction),
            0x20 => self.ins_lb(instruction),
            0x21 => self.ins_lh(instruction),
            0x23 => self.ins_lw(instruction),
            0x24 => self.ins_lbu(instruction),
            0x25 => self.ins_lhu(instruction),
            0x28 => self.ins_sb(instruction),
            0x29 => self.ins_sh(instruction),
            0x2b => self.ins_sw(instruction),
            _ => panic!(
                "Unimplemented opcode: {:02x} @ {:08x}",
                instruction.opcode(),
                self.pc - 4
            ),
        }
    }

    /// Calculate the effective address for a load/store instruction
    fn target_address(&self, instr: Instruction) -> u32 {
        let offset = instr.simm16() as u32;
        let rs_value = self.get_rs(instr);
        rs_value.wrapping_add(offset)
    }

    /// Get the value of the GPR register pointed to by rt
    fn get_rt(&self, instr: Instruction) -> u32 {
        self.registers[instr.rt()]
    }

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

    /// Read a value from memory.
    pub fn read_memory(&self, address: u32, size: AccessSize) -> Result<u32, ()> {
        Ok(self.ram.read(address, size))
    }

    /// Write a value to memory.
    pub fn write_memory(&mut self, address: u32, value: u32, size: AccessSize) -> Result<(), ()> {
        self.ram.write(address, value, size);
        Ok(())
    }

    /// Raises an exception (stub for now)
    fn exception(&mut self, code: &str) {
        panic!("Exception raised: {code}");
    }
}
