//[ new-logic
use crate::cpu::{Cpu, Instruction};

impl Cpu {
    //[ slti-stubs
    /// 0A - SLTI - I-type
    /// SLTI rt, rs, immediate
    /// GPR[rt] = (signed(GPR[rs]) < sign_extend(immediate)) ? 1 : 0
    pub fn ins_slti(&mut self, instruction: Instruction) {
        // Your implementation here
    }

    /// 0B - SLTIU - I-type
    /// SLTIU rt, rs, immediate
    /// GPR[rt] = (GPR[rs] < unsigned(sign_extend(immediate))) ? 1 : 0
    pub fn ins_sltiu(&mut self, instruction: Instruction) {
        // Your implementation here
    }
    //] slti-stubs

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

    //[ logic-lui
    /// 0F - LUI - I-Type
    /// LUI rt, immediate
    /// GPR[rt] = immediate << 16
    pub(super) fn ins_lui(&mut self, instr: Instruction) {
        // Your implementation here
    }
    //] logic-lui
}
//] new-logic
