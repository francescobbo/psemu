use super::Cpu;

impl Cpu {
    /// 09 - ADDIU - I-type
    /// ADDIU rt, rs, immediate
    /// GPR[rt] = GPR[rs] + sign_extended(immediate_value)
    ///
    /// No overflow exception
    pub(super) fn ins_addiu(&mut self, instruction: u32) {
        // Extract the source and destination register indices
        let rs = ((instruction >> 21) & 0x1F) as usize;
        let rt = ((instruction >> 16) & 0x1F) as usize;

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
    use super::*;

    #[test]
    fn test_addiu() {
        let mut cpu = Cpu::new();
        cpu.registers[7] = 1;
        cpu.execute(0x24e8_04D2); // ADDIU r8, r7, 1234
        assert_eq!(cpu.registers[8], 1235);

        // Writes to r0 must be ignored
        cpu.execute(0x24e0_000f); // ADDIU r0, r7, 15
        assert_eq!(cpu.registers[0], 0);

        // Overflow test
        cpu.registers[7] = 0xffff_ffff; // Set r7 to -1
        cpu.execute(0x24e8_0001); // ADDIU r8, r7, 1
        assert_eq!(cpu.registers[8], 0); // Wraps around to 0

        // Negative immediate test
        cpu.registers[7] = 0;
        cpu.execute(0x24e8_ffff); // ADDIU r8, r7, -1
        assert_eq!(cpu.registers[8], 0xffff_ffff);
    }
}
