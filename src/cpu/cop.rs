use crate::{
    bus::AccessSize,
    cpu::{Cpu, Instruction, memory::AccessType},
};

use super::control_types::ExceptionCause;

impl Cpu {
    /// 00.0C - SYSCALL
    /// Triggers a Syscall exception
    pub fn ins_syscall(&mut self, _instruction: Instruction) {
        self.exception(ExceptionCause::Syscall, self.current_pc);
    }

    /// 00.0D - BREAK
    /// Triggers a Breakpoint exception
    pub fn ins_break(&mut self, _instruction: Instruction) {
        self.exception(ExceptionCause::Breakpoint, self.current_pc);
    }

    /// 10.00 - MFC0 - R-Type (kind of)
    /// MFC0 rt, rd
    /// GPR[rt] = COP0[rd]
    pub(super) fn ins_mfc0(&mut self, instruction: Instruction) {
        if let Some(value) = self.cop0.read(instruction.rd()) {
            self.delayed_load(instruction.rt(), value)
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
        self.cop_delayed_load(0, instruction.rd(), self.get_rt(instruction));
    }

    /// 10.06 - CTC0 - R-Type (kind of)
    /// CTC0 rt, rd
    /// COP0[rd + 32] = GPR[rt]
    /// See `ins_cfc0`
    pub(super) fn ins_ctc0(&mut self, _instruction: Instruction) {
        panic!("CTC0 instruction is not supported on PS1");
    }

    /// 12.00 - MFC2 - R-Type (kind of)
    /// MFC2 rt, rd
    /// GPR[rt] = COP2[rd]
    pub(super) fn ins_mfc2(&mut self, instruction: Instruction) {
        if let Some(value) = self.gte.read(instruction.rd()) {
            self.delayed_load(instruction.rt(), value)
        } else {
            panic!("Invalid GTE register read: {}", instruction.rd());
        }
    }

    /// 12.02 - CFC2 - R-Type (kind of)
    /// CFC2 rt, rd
    /// GPR[rt] = COP2[rd + 32]
    pub(super) fn ins_cfc2(&mut self, instruction: Instruction) {
        if let Some(value) = self.gte.read(instruction.rd() + 32) {
            self.write_reg(instruction.rt(), value);
        } else {
            panic!("Invalid GTE register read: {}", instruction.rd());
        }
    }

    /// 12.04 - MTC2 - R-Type (kind of)
    /// MTC2 rt, rd
    /// COP2[rd] = GPR[rt]
    pub(super) fn ins_mtc2(&mut self, instruction: Instruction) {
        self.cop_delayed_load(2, instruction.rd(), self.get_rt(instruction));
    }

    /// 12.06 - CTC2 - R-Type (kind of)
    /// CTC2 rt, rd
    /// COP2[rd + 32] = GPR[rt]
    pub(super) fn ins_ctc2(&mut self, instruction: Instruction) {
        self.cop_delayed_load(
            2,
            instruction.rd() + 32,
            self.get_rt(instruction),
        );
    }

    pub fn ins_lwc2(&mut self, instruction: Instruction) {
        let address = self.target_address(instruction);
        match self.read_memory(address, AccessSize::Word) {
            Ok(value) => {
                self.gte.write(instruction.rt(), value).unwrap();
            }
            Err(e) => {
                self.memory_access_exception(e, AccessType::Read, address, self.current_pc);
            }
        };
    }

    pub fn ins_swc2(&mut self, instruction: Instruction) {
        let address = self.target_address(instruction);
        let value = self.gte.read(instruction.rt()).unwrap();

        match self.write_memory(address, value, AccessSize::Word) {
            Ok(_) => {}
            Err(e) => {
                self.memory_access_exception(e, AccessType::Write, address, self.current_pc);
            }
        }
    }
}
