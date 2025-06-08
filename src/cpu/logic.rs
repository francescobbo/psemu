//[ new-logic
use crate::cpu::{Cpu, Instruction};

impl Cpu {
    /// 0C - ANDI - I-Type
    /// ANDI rt, rs, immediate
    /// GPR[rt] = GPR[rs] & immediate
    pub(super) fn ins_andi(&mut self, instr: Instruction) {
        // Your implementation here
    }

    /// 0D - ORI - I-Type
    /// ORI rt, rs, immediate
    /// GPR[rt] = GPR[rs] | immediate
    pub(super) fn ins_ori(&mut self, instr: Instruction) {
        // Your implementation here
    }

    /// 0E - XORI - I-Type
    /// XORI rt, rs, immediate
    /// GPR[rt] = GPR[rs] ^ immediate
    pub(super) fn ins_xori(&mut self, instr: Instruction) {
        // Your implementation here
    }
}
//] new-logic
