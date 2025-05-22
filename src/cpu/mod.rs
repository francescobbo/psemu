mod arith;
mod branch;
mod instruction;
mod load_store;
mod logic;
#[cfg(test)]
mod test_utils;

use crate::{
    ram::{AccessSize, Ram},
};
pub use instruction::Instruction;


const NUM_REGISTERS: usize = 32;

pub struct Cpu {
    /// The CPU's general-purpose registers.
    pub registers: [u32; NUM_REGISTERS],

    /// The program counter, which points to the next instruction
    pub pc: u32,

    /// RAM instance to access memory.
    pub ram: Ram,

    /// The target of a branch instruction, which will be placed in PC after the
    /// delay slot.
    pub branch_target: Option<u32>,

    /// The target of a branch instruction that will be reached after the
    /// current instruction is executed.
    pub current_branch_target: Option<u32>,
}

impl Cpu {
    pub fn new() -> Self {
        Cpu {
            ram: Ram::new(),
            registers: [0; NUM_REGISTERS],
            pc: 0,
            branch_target: None,
            current_branch_target: None,
        }
    }

    /// Perform one step of the CPU cycle.
    pub fn step(&mut self) {
        // Take the target of the branch instruction, if we are in a delay slot.
        self.current_branch_target = self.branch_target.take();

        // Fetch the instruction at the current program counter (PC).
        // This may be a delay slot instruction.
        let instruction = self.fetch_instruction(self.pc);

        // Update PC to point to the following instruction.
        self.pc += 4;

        // Execute the instruction based on its opcode and function code.
        self.execute(instruction);

        // If the instruction we just executed was a delay slot - and the slot
        // did not contain a branch instruction (creating a new delay slot) -
        // then we need to update the PC to point to the target of the branch.
        if self.branch_target.is_none() {
            if let Some(target) = self.current_branch_target.take() {
                self.pc = target;
            }
        }
    }

    /// Fetch the instruction from the given address.
    fn fetch_instruction(&self, address: u32) -> Instruction {
        Instruction(self.read_memory(address, AccessSize::Word).unwrap())
    }

    /// Execute an instruction
    fn execute(&mut self, instruction: Instruction) {
        match instruction.opcode() {
            0x00 => {
                // R-type instructions
                match instruction.funct() {
                    0x00 => self.ins_sll(instruction),
                    0x02 => self.ins_srl(instruction),
                    0x03 => self.ins_sra(instruction),
                    0x04 => self.ins_sllv(instruction),
                    0x06 => self.ins_srlv(instruction),
                    0x07 => self.ins_srav(instruction),
                    0x08 => self.ins_jr(instruction),
                    0x09 => self.ins_jalr(instruction),
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
                    _ => {
                        println!(
                            "Unimplemented funct: {:02x} @ {:08x}",
                            instruction.funct(),
                            self.pc - 4
                        );
                        self.exception("Unimplemented funct");
                    }
                }
            }
            0x01 => {
                // This format abuses the `rt` field for a sub-opcode
                match instruction.rt() {
                    0x00 => self.ins_bltz(instruction),
                    0x01 => self.ins_bgez(instruction),
                    0x10 => self.ins_bltzal(instruction),
                    0x11 => self.ins_bgezal(instruction),
                    _ => {
                        println!("Unimplemented funct: {:02x} @ {:08x}", instruction.rt(), self.pc - 4);
                        self.exception("Unimplemented funct");
                    }
                }
            }
            0x02 => self.ins_j(instruction),
            0x03 => self.ins_jal(instruction),
            0x04 => self.ins_beq(instruction),
            0x05 => self.ins_bne(instruction),
            0x06 => self.ins_blez(instruction),
            0x07 => self.ins_bgtz(instruction),
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
            _ => {
                println!(
                    "Unimplemented opcode: {:02x} @ {:08x}",
                    instruction.funct(),
                    self.pc - 4
                );
                self.exception("Unimplemented funct");
            }
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
