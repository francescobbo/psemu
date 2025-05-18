use super::{Cpu, Instruction};

impl Cpu {
    /// 20 - LB - I-type
    /// LB rt, offset(rs)
    /// GPR[rt] = sign_extend(Memory[rs + offset, 8-bit])
    pub(super) fn ins_lb(&mut self, instr: Instruction) {
        let address = self.target_address(instr);
        let value = self.read_memory(address, 1).unwrap() as i8 as u32;

        self.write_reg(instr.rt(), value);
    }

    /// 21 - LH - I-type
    /// LH rt, offset(rs)
    /// GPR[rt] = sign_extend(Memory[rs + offset, 16-bit])
    pub(super) fn ins_lh(&mut self, instr: Instruction) {
        let address = self.target_address(instr);
        let value = self.read_memory(address, 2).unwrap() as i16 as u32;

        self.write_reg(instr.rt(), value);
    }

    /// 23 - LW - I-type
    /// LW rt, offset(rs)
    /// GPR[rt] = Memory[rs + offset, 32-bit]
    pub(super) fn ins_lw(&mut self, instr: Instruction) {
        let address = self.target_address(instr);
        let value = self.read_memory(address, 4).unwrap();

        self.write_reg(instr.rt(), value);
    }

    /// 24 - LBU - I-type
    /// LBU rt, offset(rs)
    /// GPR[rt] = zero_extend(Memory[rs + offset, 8-bit])
    pub(super) fn ins_lbu(&mut self, instr: Instruction) {
        let address = self.target_address(instr);
        let value = self.read_memory(address, 1).unwrap();

        self.write_reg(instr.rt(), value);
    }

    /// 25 - LHU - I-type
    /// LHU rt, offset(rs)
    /// GPR[rt] = zero_extend(Memory[rs + offset, 16-bit])
    pub(super) fn ins_lhu(&mut self, instr: Instruction) {
        let address = self.target_address(instr);
        let value = self.read_memory(address, 2).unwrap();

        self.write_reg(instr.rt(), value);
    }

    /// 28 - SB - I-type
    /// SB rt, offset(rs)
    /// Memory[rs + sign_extended(offset), 8-bit] = GPR[rt]
    pub(super) fn ins_sb(&mut self, instr: Instruction) {
        let address = self.target_address(instr);
        let value = self.get_rt(instr);

        self.write_memory(address, value, 1).unwrap();
    }

    /// 29 - SH - I-type
    /// SH rt, offset(rs)
    /// Memory[rs + sign_extended(offset), 16-bit] = GPR[rt]
    pub(super) fn ins_sh(&mut self, instr: Instruction) {
        let address = self.target_address(instr);
        let value = self.get_rt(instr);

        self.write_memory(address, value, 2).unwrap();
    }

    /// 2B - SW - I-type
    /// SW rt, offset(rs)
    /// Memory[rs + sign_extended(offset), 32-bit] = GPR[rt]
    pub(super) fn ins_sw(&mut self, instr: Instruction) {
        let address = self.target_address(instr);
        let value = self.get_rt(instr);

        self.write_memory(address, value, 4).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use crate::cpu::test_utils::*;

    #[test]
    fn test_sw() {
        let mut cpu = test_cpu(
            &[(7, 0x1000), (8, 0x12345678)],
            &[
                // SW r8, 0(r7)
                i_type(0x2b, 8, 7, 0),
                // SW r8, -4(r7)
                i_type(0x2b, 8, 7, 0xfffc),
            ],
        );

        cpu_steps(&mut cpu, 2);
        assert_eq!(cpu.read_memory(0x1000, 4).unwrap(), 0x12345678);
        assert_eq!(cpu.read_memory(0x0ffc, 4).unwrap(), 0x12345678);

        // Test that the store was done in little-endian order
        assert_eq!(cpu.read_memory(0x1000, 2).unwrap(), 0x5678);
    }
}
