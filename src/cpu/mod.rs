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

#[derive(Default)]
pub struct Cpu {
    /// The Program Counter: holds the address of the instruction to be fetched next.
    pub pc: u32,

    /// Next PC: on a taken branch, holds the target address, otherwise, it's `pc + 4`.
    pub npc: u32,

    /// The address of the instruction currently being executed. Used only to
    /// set the EPC register when an exception occurs.
    pub current_instruction_pc: u32,

    /// The CPU's general-purpose registers.
    pub registers: [u32; NUM_REGISTERS],

    /// The HI register
    pub hi: u32,

    /// The LO register
    pub lo: u32,

    /// The branch state for the instruction to be executed in the next cycle.
    pub next_branch_state: Option<bool>,

    /// The branch state for the instruction executing in the current cycle.
    pub active_branch_state: Option<bool>,

    /// A load instruction has just executed and scheduled this operation.
    /// It will become `active_load_delay` in the next cycle.
    pub scheduled_load: Option<DelayedLoad>,

    /// This is a load from the previous cycle. It's now active in the
    /// delay slot and will be committed at the end of the current cycle.
    pub active_load_delay: Option<DelayedLoad>,

    /// Number of cycles consumed by the current step.
    pub step_cycles: usize,

    /// I/O bus that connects the CPU to the rest of the system.
    pub bus: Bus,

    /// The COP0 coprocessor, which handles system control operations.
    pub cop0: control::Cop0,

    // The COP2 coprocessor (the GTE), which handles graphics transformations.
    pub gte: gte::Gte,

    /// Used by the debugger to trigger memory access breakpoints.
    pub last_memory_operation: (AccessType, u32),
}

/// Represents a delayed load operation.
#[derive(Debug, Clone, Copy, Default)]
pub struct DelayedLoad {
    /// The register to load to
    pub target: usize,

    /// The value to load
    pub value: u32,

    /// The coprocessor number (if applicable)
    pub coprocessor: Option<u8>,
}

impl Cpu {
    pub fn new() -> Self {
        Cpu {
            pc: 0xbfc0_0000,
            npc: 0xbfc0_0004,
            ..Default::default()
        }
    }

    /// Perform one step of the CPU cycle.
    pub fn step(&mut self) -> usize {
        // Take the load delay, if we have one.
        self.active_load_delay = self.scheduled_load.take();
        self.active_branch_state = self.next_branch_state.take();

        self.current_instruction_pc = self.pc;
        self.step_cycles = 0;

        // Fetch the instruction at the current program counter (PC).
        // This may be a delay slot instruction.
        let instruction =
            match self.fetch_instruction(self.current_instruction_pc) {
                Ok(value) => value,
                Err(err) => {
                    // If we failed to fetch the instruction, we handle the error
                    self.memory_access_exception(
                        err,
                        AccessType::InstructionFetch,
                        self.current_instruction_pc,
                        Instruction(0),
                    );
                    return 10;
                }
            };

        if self.cop0.should_interrupt() {
            if instruction.opcode() == 0x12 && instruction.cop_execute() {
                // GTE execute instructions are handled even if an interrupt
                // is pending. This is likely a CPU bug, but some games rely on
                // it.
                self.gte.execute(instruction);
            }

            // If the coprocessor requests an interrupt, we handle it
            self.handle_load_delay();
            self.exception(ExceptionCause::Interrupt, instruction);
            return 10;
        }

        // Update PC and NPC.
        self.pc = self.npc; // Set the PC to the next instruction
        self.npc = self.npc.wrapping_add(4); // Prepare the next PC

        // Execute the instruction based on its opcode and function code.
        self.execute(instruction);

        // If we have a delayed load, we need to write the value to the target
        self.handle_load_delay();

        self.step_cycles
    }

    /// Fetch the instruction from the given address.
    fn fetch_instruction(
        &mut self,
        address: u32,
    ) -> Result<Instruction, MemoryError> {
        match address {
            0..=0x9fffffff => {
                // cached instruction
                self.step_cycles += 2;
            }
            0xa0000000..=0xafffffff => {
                // uncached instruction from memory
                self.step_cycles += 5;
            }
            0xb0000000.. => {
                // uncached instruction from ROM (probably)
                self.step_cycles += 24;
            }
        }

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
                        self.exception(
                            ExceptionCause::ReservedInstruction,
                            instruction,
                        );
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
            0x11 => {
                println!(
                    "[CPU] Unimplemented coprocessor 1 instruction: {:08x}",
                    instruction.0
                );
            }
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
            0x13 => {
                println!(
                    "[CPU] Unimplemented coprocessor 3 instruction: {:08x}",
                    instruction.0
                );
            }
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
            0x30 => {
                println!(
                    "[CPU] LWC0 does not exist on the PS1"
                );
            }
            0x31 => {
                println!(
                    "[CPU] LWC0 does not exist on the PS1"
                );
            }
            0x32 => self.ins_lwc2(instruction),
            0x33 => {
                println!(
                    "[CPU] LWC0 does not exist on the PS1"
                );
            }
            0x3a => self.ins_swc2(instruction),
            0x38..=0x3b => {
                println!(
                    "[CPU] Unimplemented instruction: {:08x}",
                    instruction.0
                );
            }
            _ => {
                self.exception(
                    ExceptionCause::ReservedInstruction,
                    instruction,
                );
            }
        }
    }

    /// Calculate the effective address for a load/store instruction
    fn target_address(&self, instruction: Instruction) -> u32 {
        let offset = instruction.simm16() as u32;
        let rs_value = self.get_rs(instruction);
        rs_value.wrapping_add(offset)
    }

    /// Get the value of the GPR register pointed to by rt
    fn get_rt(&self, instruction: Instruction) -> u32 {
        self.registers[instruction.rt()]
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
        if let Some(load) = self.active_load_delay.take() {
            match load.coprocessor {
                Some(0) => {
                    // COP0 delayed load
                    self.cop0
                        .write(load.target, load.value)
                        .expect("Failed to write COP0 delayed load");
                }
                Some(2) => {
                    // GTE delayed load
                    self.gte
                        .write(load.target, load.value)
                        .expect("Failed to write GTE delayed load");
                }
                None => {
                    // Write the value to the target register
                    self.registers[load.target] = load.value;
                }
                _ => {
                    panic!(
                        "Invalid coprocessor for delayed load: {:?}",
                        load.coprocessor
                    );
                }
            }
        }
    }

    pub(super) fn exception(
        &mut self,
        cause: ExceptionCause,
        instruction: Instruction,
    ) {
        let in_bds = self.active_branch_state.is_some();
        let branch_taken = self.active_branch_state == Some(true);

        let coprocessor_number = instruction.opcode() & 0x3;

        self.pc = self.cop0.start_exception(
            cause,
            self.current_instruction_pc,
            self.pc,
            in_bds,
            branch_taken,
            coprocessor_number,
        );
        self.npc = self.pc.wrapping_add(4);
    }
}
