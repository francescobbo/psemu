//[ new-file
use super::{AccessSize, Cpu, Instruction};

impl Cpu {
    /// 23 - LW - I-type
    /// LW rt, offset(rs)
    /// GPR[rt] = Memory[rs + sign_extend(offset)]
    pub(super) fn ins_lw(&mut self, instr: Instruction) {
        let address = self.target_address(instr);

        match self.read_memory(address, AccessSize::Word) {
            Ok(value) => self.write_reg(instr.rt(), value),
            Err(_) => self.exception("Memory read error"),
        }
    }
}
//] new-file
