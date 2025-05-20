mod arith;
mod branch;
mod instruction;
mod load_store;
mod logic;

use crate::ram::Ram;
pub use instruction::Instruction;

const NUM_REGISTERS: usize = 32;

#[derive(Debug)]
pub struct Cpu {
    /// RAM instance to access memory.
    pub ram: Ram,

    /// The CPU's general-purpose registers.
    pub registers: [u32; NUM_REGISTERS],

    /// The HI register
    pub hi: u32,

    /// The LO register
    pub lo: u32,

    /// The program counter, which points to the next instruction
    pub pc: u32,

    /// The target of a branch instruction, which will be placed in PC after the
    /// delay slot.
    pub next_pc: Option<u32>,

    /// The load delay slot: a load operation that is not completed in the same
    /// cycle as the instruction that performed it.
    pub load_delay: Option<DelayedLoad>,

    /// The index of the register written to in the current cycle, if any.
    /// This is used to ignore the load delay writeback
    pub last_written_register: usize,
}

#[derive(Debug)]
/// Represents a delayed load operation.
pub struct DelayedLoad {
    /// The register to load to
    pub target: usize,

    /// The value to load
    pub value: u32,
}

impl Cpu {
    pub fn new() -> Self {
        Cpu {
            ram: Ram::new(),
            registers: [0; NUM_REGISTERS],
            hi: 0,
            lo: 0,
            pc: 0,
            next_pc: None,
            load_delay: None,
            last_written_register: 0,
        }
    }

    /// Perform one step of the CPU cycle.
    pub fn step(&mut self) {
        // Take the target of the branch instruction, if we are in a delay slot.
        let jump_target = self.next_pc.take();
        // Take the load delay, if we have one.
        let delayed_load = self.load_delay.take();

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
        if let Some(target) = jump_target
            && self.next_pc.is_none()
        {
            self.pc = target;
        }

        // If we have a delayed load, we need to write the value to the target
        self.handle_load_delay(delayed_load);
    }

    /// Fetch the instruction from the given address.
    fn fetch_instruction(&self, address: u32) -> Instruction {
        Instruction(self.ram.read32(address))
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
                    0x10 => self.ins_mfhi(instruction),
                    0x11 => self.ins_mthi(instruction),
                    0x12 => self.ins_mflo(instruction),
                    0x13 => self.ins_mtlo(instruction),
                    0x18 => self.ins_mult(instruction),
                    0x19 => self.ins_multu(instruction),
                    0x21 => self.ins_addu(instruction),
                    0x23 => self.ins_subu(instruction),
                    0x24 => self.ins_and(instruction),
                    0x25 => self.ins_or(instruction),
                    0x26 => self.ins_xor(instruction),
                    0x27 => self.ins_nor(instruction),
                    0x2a => self.ins_slt(instruction),
                    0x2b => self.ins_sltu(instruction),
                    _ => panic!("Unimplemented funct: {:#X}", instruction.funct()),
                }
            }
            0x01 => {
                // This format abuses the `rt` field for a sub-opcode
                match instruction.rt() {
                    0x00 => self.ins_bltz(instruction),
                    0x01 => self.ins_bgez(instruction),
                    0x10 => self.ins_bltzal(instruction),
                    0x11 => self.ins_bgezal(instruction),
                    _ => panic!("Unimplemented funct: {:#X}", instruction.rt()),
                }
            }
            0x02 => self.ins_j(instruction),
            0x03 => self.ins_jal(instruction),
            0x04 => self.ins_beq(instruction),
            0x05 => self.ins_bne(instruction),
            0x06 => self.ins_blez(instruction),
            0x07 => self.ins_bgtz(instruction),
            0x09 => self.ins_addiu(instruction),
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
            _ => panic!("Unimplemented opcode: {:#X}", instruction.opcode()),
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
        self.registers[instr.rt() as usize]
    }

    /// Get the value of the GPR register pointed to by rs
    fn get_rs(&self, instruction: Instruction) -> u32 {
        self.registers[instruction.rs() as usize]
    }

    /// Write a value to a GPR register
    fn write_reg(&mut self, index: usize, value: u32) {
        // The zero register (R0) is always 0, so we don't allow writing to it
        if index == 0 {
            return;
        }

        self.registers[index] = value;

        // Remember that we wrote to this register in the current cycle
        self.last_written_register = index;
    }

    /// Completes a delayed load operation, unless it was started in the same
    /// cycle. Otherwise it marks the load for writeback in the next cycle.
    fn handle_load_delay(&mut self, delayed_load: Option<DelayedLoad>) {
        if let Some(load) = delayed_load {
            // Check if the last instruction was a direct write to the same register
            if self.last_written_register == load.target {
                // Ignore the load
                return;
            }

            // Write the value to the target register
            self.registers[load.target] = load.value;
        }

        // Reset the last written register for the next cycle
        self.last_written_register = 0;
    }
}
