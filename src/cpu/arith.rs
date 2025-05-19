use super::{Cpu, Instruction};

impl Cpu {
    /// 00.21 - ADDU - R-Type
    /// ADDU rd, rs, rt
    /// GPR[rd] = GPR[rs] + GPR[rt]
    ///
    /// No overflow exception
    pub(super) fn ins_addu(&mut self, instruction: Instruction) {
        self.write_reg(
            instruction.rd(),
            self.get_rs(instruction)
                .wrapping_add(self.get_rt(instruction)),
        );
    }

    /// 00.23 - SUBU - R-Type
    /// SUBU rd, rs, rt
    /// GPR[rd] = GPR[rs] - GPR[rt]
    ///
    /// No overflow exception
    pub(super) fn ins_subu(&mut self, instruction: Instruction) {
        self.write_reg(
            instruction.rd(),
            self.get_rs(instruction)
                .wrapping_sub(self.get_rt(instruction)),
        );
    }

    /// 09 - ADDIU - I-type
    /// ADDIU rt, rs, immediate
    /// GPR[rt] = GPR[rs] + sign_extended(immediate_value)
    ///
    /// No overflow exception
    pub(super) fn ins_addiu(&mut self, instr: Instruction) {
        let immediate = instr.simm16() as u32;
        let result = self.get_rs(instr).wrapping_add(immediate);

        self.write_reg(instr.rt(), result);
    }

    /// 0F - LUI - I-type
    /// LUI rt, immediate
    /// GPR[rt] = immediate_value << 16
    pub(super) fn ins_lui(&mut self, instr: Instruction) {
        self.write_reg(instr.rt(), instr.imm16() << 16);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_addiu() {
        let mut cpu = Cpu::new();
        cpu.registers[7] = 1;
        cpu.execute(Instruction(0x24e8_04d2)); // ADDIU r8, r7, 1234
        assert_eq!(cpu.registers[8], 1235);

        // Writes to r0 must be ignored
        cpu.execute(Instruction(0x24e0_000f)); // ADDIU r0, r7, 15
        assert_eq!(cpu.registers[0], 0);

        // Overflow test
        cpu.registers[7] = 0xffff_ffff; // Set r7 to -1
        cpu.execute(Instruction(0x24e8_0001)); // ADDIU r8, r7, 1
        assert_eq!(cpu.registers[8], 0); // Wraps around to 0

        // Negative immediate test
        cpu.registers[7] = 0;
        cpu.execute(Instruction(0x24e8_ffff)); // ADDIU r8, r7, -1
        assert_eq!(cpu.registers[8], 0xffff_ffff);
    }

    #[test]
    fn test_lui() {
        let mut cpu = Cpu::new();
        cpu.execute(Instruction(0x3c08_1234)); // LUI r8, 0x1234
        assert_eq!(cpu.registers[8], 0x1234_0000);

        // Writes to r0 must be ignored
        cpu.execute(Instruction(0x3c00_5678)); // LUI r0, 0x5678
        assert_eq!(cpu.registers[0], 0);
    }
}
