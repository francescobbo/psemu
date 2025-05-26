use crate::cpu::{Cpu, Instruction};

use super::control_types::ExceptionCause;

impl Cpu {
    /// 00.0C - SYSCALL
    /// Triggers a Syscall exception
    pub fn ins_syscall(&mut self, _instruction: Instruction) {
        self.pc = self.cop0.start_exception(
            ExceptionCause::Syscall,
            self.pc.wrapping_sub(4),
        );
    }

    /// 00.0D - BREAK
    /// Triggers a Breakpoint exception
    pub fn ins_break(&mut self, _instruction: Instruction) {
        self.pc = self.cop0.start_exception(
            ExceptionCause::Breakpoint,
            self.pc.wrapping_sub(4),
        );
    }

    /// 10.00 - MFC0 - R-Type (kind of)
    /// MFC0 rt, rd
    /// GPR[rt] = COP0[rd]
    pub(super) fn ins_mfc0(&mut self, instruction: Instruction) {
        if let Some(value) = self.cop0.read(instruction.rd()) {
            self.write_reg(instruction.rt(), value);
        } else {
            panic!("Invalid COP0 register read: {}", instruction.rd());
        }
    }

    /// 10.02 - CFC0 - R-Type (kind of)
    /// CFC0 rt, rd
    /// GPR[rt] = COP0[rd + 32]
    /// This is guaranteed to fail on the PS1, as there's no COP0 control registers.
    pub(super) fn ins_cfc0(&mut self, _instruction: Instruction) {
        panic!("CFC0 instruction is not supported on PS1");
    }

    /// 10.04 - MTC0 - R-Type (kind of)
    /// MTC0 rt, rd
    /// COP0[rd] = GPR[rt]
    pub(super) fn ins_mtc0(&mut self, instruction: Instruction) {
        if let Err(_) = self.cop0.write(instruction.rd(), self.get_rt(instruction)) {
            panic!("Invalid COP0 register write: {}", instruction.rd());
        }
    }

    /// 10.06 - CTC0 - R-Type (kind of)
    /// CTC0 rt, rd
    /// COP0[rd + 32] = GPR[rt]
    /// See `ins_cfc0`
    pub(super) fn ins_ctc0(&mut self, _instruction: Instruction) {
        panic!("CTC0 instruction is not supported on PS1");
    }
}
