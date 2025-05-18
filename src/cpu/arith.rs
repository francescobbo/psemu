use super::{Cpu, Instruction};

impl Cpu {
    /// 08 - ADDI - I-type
    /// ADDIU rt, rs, immediate
    /// GPR[rt] = GPR[rs] + sign_extended(immediate_value)
    ///
    /// Causes overflow exception if the result is not representable in 32 bits
    pub(super) fn ins_addi(&mut self, instr: Instruction) {
        let immediate = instr.simm16();
        let result = self.get_rs(instr) as i32 + immediate;

        self.write_reg(instr.rt(), result as u32);
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
    use crate::cpu::test_utils::*;

    #[test]
    fn test_addi() {
        let mut cpu = test_cpu(
            &[(7, 1)],
            &[
                // ADDI r8, r7, 0
                i_type(0x08, 8, 7, 0),
                // ADDI r9, r7, 1234
                i_type(0x08, 9, 7, 1234),
                // ADDI r10, r7, 15
                i_type(0x08, 0, 7, 0xffff),
                // ADDI r0, r7, 15
                i_type(0x08, 0, 7, 15),
            ],
        );
        cpu_steps(&mut cpu, 3);

        assert_eq!(cpu.registers[8], 1);
        assert_eq!(cpu.registers[9], 1 + 1234);
        assert_eq!(cpu.registers[10], 0); // subtraction does not overflow
        assert_eq!(cpu.registers[0], 0);
    }

    #[test]
    #[should_panic]
    fn test_addi_overflow() {
        let mut cpu = test_cpu(
            &[(7, 0x7fff_ffff)],
            &[
                // ADDI r8, r7, 1
                i_type(0x08, 8, 7, 1),
            ],
        );
        cpu.step();

        // This should panic due to overflow as 7fff_ffff is the largest signed
        // 32-bit integer, and adding 1 would take us to the largest signed
        // negative integer.
    }

    #[test]
    fn test_addiu() {
        let mut cpu = test_cpu(
            &[(1, 0x1000), (2, 0xffff_ffff)],
            &[
                // ADDIU r3, r1, 0
                i_type(0x09, 3, 1, 0),
                // ADDIU r4, r1, 24
                i_type(0x09, 4, 1, 24),
                // ADDIU r5, r2, 1
                i_type(0x09, 5, 2, 1),
                // ADDIU r6, r2, -0x1001
                i_type(0x09, 6, 1, 0xefff),
                // ADDIU r0, r1, 15
                i_type(0x09, 0, 1, 0x000f),
            ],
        );

        cpu_steps(&mut cpu, 5);

        // r3 = r1 + 0
        assert_eq!(cpu.registers[3], 0x1000);
        // r4 = r1 + 24
        assert_eq!(cpu.registers[4], 0x1000 + 24);
        // r5 = r2 + 1 = 0 (wrapping)
        assert_eq!(cpu.registers[5], 0);
        // r6 = r2 - 0x1001 = 0xffff_ffff = -1
        assert_eq!(cpu.registers[6], 0xffff_ffff);
        // r0 = 0 (always zero)
        assert_eq!(cpu.registers[0], 0);
    }

    #[test]
    fn test_lui() {
        let mut cpu = test_cpu(
            &[],
            &[
                // LUI r0, 1
                i_type(0x0f, 0, 0, 1),
                // LUI r8, 0
                i_type(0x0f, 8, 0, 0),
                // LUI r9, 1234
                i_type(0x0f, 9, 0, 1234),
                // LUI r10, -1
                i_type(0x0f, 11, 0, 0xffff),
            ],
        );

        cpu_steps(&mut cpu, 4);

        assert_eq!(cpu.registers[0], 0);
        assert_eq!(cpu.registers[8], 0);
        assert_eq!(cpu.registers[9], 1234 << 16);
        assert_eq!(cpu.registers[11], 0xffff << 16);
    }
}
