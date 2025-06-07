//[ arith-new-file
use super::Cpu;

impl Cpu {
    //[ arith-ins-addiu
    /// 09 - ADDIU - I-Type
    /// ADDIU rt, rs, immediate
    /// GPR[rt] = GPR[rs] + sign_extend(immediate)
    ///
    /// No overflow exception
    pub(super) fn ins_addiu(&mut self, instruction: u32) {
        todo!("Implement ADDIU instruction");
    }
    //] arith-ins-addiu
}
//] arith-new-file
