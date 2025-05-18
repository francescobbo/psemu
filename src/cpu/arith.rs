use super::Cpu;

impl Cpu {
    /// 09 - ADDIU - I-type
    /// ADDIU rt, rs, immediate
    /// GPR[rt] = GPR[rs] + sign_extended(immediate_value)
    ///
    /// No overflow exception
    pub(super) fn ins_addiu(&mut self, instruction: u32) {
        // Extract the source and destination register indices
        let rs = ((instruction >> 21) & 0x1f) as usize;
        let rt = ((instruction >> 16) & 0x1f) as usize;

        // Extract the immediate value and sign-extend it to 32 bits
        let immediate = instruction as i16 as u32;

        // Perform the addition
        if rt != 0 {
            self.registers[rt] = self.registers[rs].wrapping_add(immediate);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::cpu::test_utils::*;

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
}
