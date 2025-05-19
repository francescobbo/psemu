use crate::cpu::{Cpu, Instruction};

impl Cpu {
    /// 02 - J - J-Type
    /// J destination
    /// PC = (PC & 0xF000_0000) | (destination << 2)
    pub(super) fn ins_j(&mut self, instruction: Instruction) {
        self.pc = (self.pc & 0xf000_0000) | (instruction.jump_target() << 2);
    }

    /// 04 - BEQ - I-Type
    /// BEQ rs, rt, offset
    /// if (GPR[rs] == GPR[rt])
    ///     PC = PC + sign_extended(offset << 2)
    pub(super) fn ins_beq(&mut self, instruction: Instruction) {
        if self.get_rs(instruction) == self.get_rt(instruction) {
            self.pc += (instruction.simm16() as u32) << 2;
        }
    }
}
