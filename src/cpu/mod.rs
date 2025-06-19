mod arith;
mod branch;
mod control;
mod control_types;
mod cop;
mod gte;
mod instruction;
mod load_store;
mod logic;
mod memory;
#[cfg(test)]
mod test_utils;

use crate::bus::{AccessSize, Bus};
use control_types::ExceptionCause;
pub use instruction::Instruction;
pub use memory::{AccessType, MemoryError};

const NUM_REGISTERS: usize = 32;

pub struct Cpu {
    /// The CPU's general-purpose registers.
    pub registers: [u32; NUM_REGISTERS],

    /// The HI register
    pub hi: u32,

    /// The LO register
    pub lo: u32,

    /// The program counter, which points to the next instruction
    pub pc: u32,

    /// I/O bus that connects the CPU to the rest of the system.
    pub bus: Bus,

    /// The target of a branch instruction, which will be placed in PC after the
    /// delay slot.
    pub branch_target: Option<u32>,

    /// The target of a branch instruction that will be reached after the
    /// current instruction is executed.
    pub current_branch_target: Option<u32>,

    /// The load delay slot: a load operation that is not completed in the same
    /// cycle as the instruction that performed it.
    pub load_delay: Option<DelayedLoad>,

    /// The delayed load operation that is currently in progress, and will be
    /// persisted at the end of the current cycle.
    pub current_load_delay: Option<DelayedLoad>,

    /// The BIU/Cache Control Register
    pub biu_cache_control: u32,

    /// The COP0 coprocessor, which handles system control operations.
    pub cop0: control::Cop0,

    // The COP2 coprocessor (the GTE), which handles graphics transformations.
    pub gte: gte::Gte,

    pub last_memory_operation: (AccessType, u32),

    pub steps: u64,
}

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
            registers: [0; NUM_REGISTERS],
            hi: 0,
            lo: 0,
            pc: 0xbfc0_0000,
            bus: Bus::new(),
            branch_target: None,
            current_branch_target: None,
            load_delay: None,
            current_load_delay: None,
            biu_cache_control: 0,
            cop0: control::Cop0::new(),
            gte: gte::Gte::new(),
            last_memory_operation: (AccessType::InstructionFetch, 0),

            steps: 0,
        }
    }

    /// Perform one step of the CPU cycle.
    pub fn step(&mut self) {
        // Take the target of the branch instruction, if we are in a delay slot.
        self.current_branch_target = self.branch_target.take();
        // Take the load delay, if we have one.
        self.current_load_delay = self.load_delay.take();

        // self.steps += 1;
        // if self.steps > 19290000 {
        //     // Print the current PC every 1000 steps
        //     println!("PC: {:08x}", self.pc);

        //     if self.pc == 0xb0 {
        //         println!("REGS: {:?}", self.registers);
        //     }
        // }

        // Fetch the instruction at the current program counter (PC).
        // This may be a delay slot instruction.
        let instruction = match self.fetch_instruction(self.pc) {
            Ok(value) => value,
            Err(err) => {
                // If we failed to fetch the instruction, we handle the error
                self.memory_access_exception(
                    err,
                    AccessType::InstructionFetch,
                    self.pc,
                );
                return;
            }
        };

        // Update PC to point to the following instruction.
        self.pc += 4;

        if self.cop0.should_interrupt() {
            // If the coprocessor requests an interrupt, we handle it
            self.handle_load_delay();
            self.exception(ExceptionCause::Interrupt);
            return;
        }

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

        // If we have a delayed load, we need to write the value to the target
        self.handle_load_delay();
    }

    /// Fetch the instruction from the given address.
    fn fetch_instruction(
        &mut self,
        address: u32,
    ) -> Result<Instruction, MemoryError> {
        Ok(Instruction(self.read_memory(address, AccessSize::Word)?))
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
                    0x0c => self.ins_syscall(instruction),
                    0x0d => self.ins_break(instruction),
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
                    _ => {
                        println!(
                            "Unimplemented funct: {:02x} @ {:08x}",
                            instruction.funct(),
                            self.pc - 4
                        );
                        self.exception(ExceptionCause::ReservedInstruction);
                    }
                }
            }
            0x01 => {
                let link = instruction.rt() & 0x1e == 0x10;

                // This format abuses the `rt` field for a sub-opcode
                match (instruction.rt() & 1, link) {
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
            0x10 => {
                if instruction.cop_execute() {
                    // Coprocessor 0 instructions
                    self.cop0.execute(instruction);
                } else {
                    match instruction.cop_funct() {
                        0 => self.ins_mfc0(instruction),
                        2 => self.ins_cfc0(instruction),
                        4 => self.ins_mtc0(instruction),
                        6 => self.ins_ctc0(instruction),
                        _ => panic!(
                            "Unimplemented cop0 funct: {:#x}",
                            instruction.rs()
                        ),
                    }
                }
            }
            0x11 => panic!("COP1 is not present on PS1"),
            0x12 => {
                if instruction.cop_execute() {
                    // GTE instructions
                    self.gte.execute(instruction);
                } else {
                    match instruction.cop_funct() {
                        0 => self.ins_mfc2(instruction),
                        2 => self.ins_cfc2(instruction),
                        4 => self.ins_mtc2(instruction),
                        6 => self.ins_ctc2(instruction),
                        _ => panic!(
                            "Unimplemented cop2 funct: {:#x}",
                            instruction.rs()
                        ),
                    }
                }
            }
            0x13 => panic!("COP3 is not present on PS1"),
            0x20 => self.ins_lb(instruction),
            0x21 => self.ins_lh(instruction),
            0x22 => self.ins_lwl(instruction),
            0x23 => self.ins_lw(instruction),
            0x24 => self.ins_lbu(instruction),
            0x25 => self.ins_lhu(instruction),
            0x26 => self.ins_lwr(instruction),
            0x28 => self.ins_sb(instruction),
            0x29 => self.ins_sh(instruction),
            0x2a => self.ins_swl(instruction),
            0x2b => self.ins_sw(instruction),
            0x2e => self.ins_swr(instruction),
            0x32 => self.ins_lwc2(instruction),
            0x3a => self.ins_swc2(instruction),
            _ => {
                println!(
                    "Unimplemented opcode: {:02x} @ {:08x}",
                    instruction.opcode(),
                    self.pc - 4
                );
                self.exception(ExceptionCause::ReservedInstruction);
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
        if index == 0 {
            return;
        }

        self.cancel_delayed_load(index);
        self.registers[index] = value;
    }

    /// Completes a delayed load operation, unless it was started in the same
    /// cycle. Otherwise it marks the load for writeback in the next cycle.
    fn handle_load_delay(&mut self) {
        if let Some(load) = self.current_load_delay.take() {
            // Write the value to the target register
            self.registers[load.target] = load.value;
        }
    }

    pub(super) fn exception(&mut self, cause: ExceptionCause) {
        let is_branch_delay_slot = self.current_branch_target.is_some();

        // If there was a branch target, we ignore it
        self.current_branch_target = None;

        self.pc = self.cop0.start_exception(
            cause,
            self.pc.wrapping_sub(4),
            is_branch_delay_slot,
        );
    }
}
