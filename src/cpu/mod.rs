mod arith;
mod instruction;
//[ mod-load-store
mod load_store;
//] mod-load-store
//[ mod-logic
mod logic;
//] mod-logic
#[cfg(test)]
mod test_utils;

//[ !omit
use crate::ram::Ram;
//] !omit
use instruction::Instruction;

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
        //[ ins-disasm
        // Fetch the instruction at the current program counter (PC).
        let instruction = self.fetch_instruction(self.pc).unwrap();

        // Update the program counter to point to the next instruction.
        self.pc += 4;
        //] ins-disasm

        // Execute the instruction based on its opcode and function code.
        self.execute(instruction);
    }

    fn fetch_instruction(&self, address: u32) -> Result<Instruction, ()> {
        let value = self.read_memory(address, AccessSize::Word)?;
        Ok(Instruction(value))
    }

    fn execute(&mut self, instruction: Instruction) {
        let opcode = instruction.opcode();
        //[ ins-opcodes
        match opcode {
            0x00 => match instruction.funct() {
                0x00 => self.ins_sll(instruction),
                0x02 => self.ins_srl(instruction),
                0x03 => self.ins_sra(instruction),
                0x04 => self.ins_sllv(instruction),
                0x06 => self.ins_srlv(instruction),
                0x07 => self.ins_srav(instruction),
                0x20 => self.ins_add(instruction),
                0x21 => self.ins_addu(instruction),
                0x22 => self.ins_sub(instruction),
                0x23 => self.ins_subu(instruction),
                0x24 => self.ins_and(instruction),
                0x25 => self.ins_or(instruction),
                0x26 => self.ins_xor(instruction),
                0x27 => self.ins_nor(instruction),
                0x2a => self.ins_slt(instruction),
                0x2b => self.ins_sltu(instruction),
                _ => self.exception(&format!(
                    "Unimplemented funct {:02x}",
                    instruction.funct()
                )),
            },
            0x08 => self.ins_addi(instruction),
            //] ins-opcodes
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
            //[ ins-unimplemented
            0x2b => self.ins_sw(instruction),
            _ => self.exception(&format!("Unimplemented opcode {opcode:02x}")),
        }
        //] ins-unimplemented
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
    //[ cpu-exception
    /// Mock implementation of an exception handler
    fn exception(&self, code: &str) {
        panic!("Exception raised: {code}");
    }
    //] cpu-exception
    //[ ins-helpers
    /// Get the value of the GPR register pointed to by rs
    fn get_rs(&self, instruction: Instruction) -> u32 {
        self.registers[instruction.rs()]
    }

    //[ helpers-rs-target
    /// Get the value of the GPR register pointed to by rt
    fn get_rt(&self, instr: Instruction) -> u32 {
        self.registers[instr.rt()]
    }

    /// Calculate the effective address for a load/store instruction
    fn target_address(&self, instr: Instruction) -> u32 {
        let offset = instr.simm16() as u32;
        let rs_value = self.get_rs(instr);
        rs_value.wrapping_add(offset)
    }
    //] helpers-rs-target

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
