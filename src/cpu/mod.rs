mod arith;
mod branch;
mod instruction;
mod load_store;
mod logic;
#[cfg(test)]
mod test_utils;

use crate::ram::Ram;
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
    pub hi: u32,
    pub lo: u32,
    pub pc: u32,

    pub ram: Ram,

    /// The target address for branch instructions, if applicable
    pub branch_target: Option<u32>,
    pub current_branch_target: Option<u32>,
}

impl Cpu {
    pub fn new() -> Self {
        Cpu {
            registers: [0; NUM_REGISTERS],
            hi: 0,
            lo: 0,
            pc: 0,
            ram: Ram::new(),
            branch_target: None,
            current_branch_target: None,
        }
    }

    pub fn step(&mut self) {
        self.current_branch_target = self.branch_target.take();

        // Fetch the instruction at the current program counter (PC).
        let instruction = self.fetch_instruction(self.pc).unwrap();

        println!(
            "[{:08x}] {}",
            self.pc,
            psdisasm::Disasm::new().disasm(instruction.0, self.pc)
        );

        // Update the program counter to point to the next instruction.
        self.pc += 4;

        // Execute the instruction based on its opcode and function code.
        self.execute(instruction);

        // If a branch target was set, update the PC to that target.
        if let Some(target) = self.current_branch_target.take() {
            self.pc = target;
        }
    }

    fn fetch_instruction(&self, address: u32) -> Result<Instruction, ()> {
        let value = self.read_memory(address, AccessSize::Word)?;
        Ok(Instruction(value))
    }

    fn execute(&mut self, instruction: Instruction) {
        let opcode = instruction.opcode();
        match opcode {
            0x00 => match instruction.funct() {
                0x00 => self.ins_sll(instruction),
                0x02 => self.ins_srl(instruction),
                0x03 => self.ins_sra(instruction),
                0x04 => self.ins_sllv(instruction),
                0x06 => self.ins_srlv(instruction),
                0x07 => self.ins_srav(instruction),
                0x08 => self.ins_jr(instruction),
                0x09 => self.ins_jalr(instruction),
                0x10 => self.ins_mfhi(instruction),
                0x11 => self.ins_mthi(instruction),
                0x12 => self.ins_mflo(instruction),
                0x13 => self.ins_mtlo(instruction),
                0x18 => self.ins_mult(instruction),
                0x19 => self.ins_multu(instruction),
                0x1a => self.ins_div(instruction),
                0x1b => self.ins_divu(instruction),
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
            0x01 => {
                let funct = instruction.rt();
                let link = funct & 0x1e == 0x10;

                match (funct & 1, link) {
                    (0, false) => self.ins_bltz(instruction),
                    (0, true) => self.ins_bltzal(instruction),
                    (1, false) => self.ins_bgez(instruction),
                    (1, true) => self.ins_bgezal(instruction),
                    _ => unreachable!(),
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
            _ => self.exception(&format!("Unimplemented opcode {opcode:02x}")),
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
    /// Mock implementation of an exception handler
    fn exception(&self, code: &str) {
        panic!("Exception raised: {code}");
    }
    /// Get the value of the GPR register pointed to by rs
    fn get_rs(&self, instruction: Instruction) -> u32 {
        self.registers[instruction.rs()]
    }

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

    /// Write a value to a GPR register
    fn write_reg(&mut self, index: usize, value: u32) {
        // The zero register (R0) is always 0, so we don't allow writing to it
        if index != 0 {
            self.registers[index] = value;
        }
    }
}
