use super::{Cpu, Instruction};

impl Cpu {
    /// 00.10 - MFHI - R-Type
    /// MFHI rd
    /// GPR[rd] = HI
    pub fn ins_mfhi(&mut self, instruction: Instruction) {
        self.write_reg(instruction.rd(), self.hi);
    }

    /// 00.11 - MTHI - R-Type
    /// MTHI rs
    /// HI = GPR[rs]
    pub fn ins_mthi(&mut self, instruction: Instruction) {
        self.hi = self.get_rs(instruction);
    }

    /// 00.12 - MFLO - R-Type
    /// MFLO rd
    /// GPR[rd] = LO
    pub fn ins_mflo(&mut self, instruction: Instruction) {
        self.write_reg(instruction.rd(), self.lo);
    }

    /// 00.13 - MTLO - R-Type
    /// MTLO rs
    /// LO = GPR[rs]
    pub fn ins_mtlo(&mut self, instruction: Instruction) {
        self.lo = self.get_rs(instruction);
    }

    /// 00.18 - MULT - R-Type
    /// MULT rs, rt
    /// result = sign_extended64(GPR[rs]) * sign_extended64(GPR[rt])
    /// HI = result[63:32]
    /// LO = result[31:0]
    pub fn ins_mult(&mut self, instruction: Instruction) {
        let rs = self.get_rs(instruction) as i32 as i64;
        let rt = self.get_rt(instruction) as i32 as i64;
        let result = rs.wrapping_mul(rt);

        self.hi = (result >> 32) as u32;
        self.lo = result as u32;
    }

    /// 00.19 - MULTU - R-Type
    /// MULTU rs, rt
    /// result = GPR[rs] * GPR[rt]
    /// HI = result[63:32]
    /// LO = result[31:0]
    pub fn ins_multu(&mut self, instruction: Instruction) {
        let rs = self.get_rs(instruction) as u64;
        let rt = self.get_rt(instruction) as u64;
        let result = rs.wrapping_mul(rt);

        self.hi = (result >> 32) as u32;
        self.lo = result as u32;
    }

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

    #[test]
    fn test_addu() {
        let mut cpu = Cpu::new();
        cpu.registers[7] = 1;
        cpu.registers[8] = 2;
        cpu.execute(Instruction(0x00107_4021)); // ADDU r8, r7, r8
        assert_eq!(cpu.registers[8], 3);
    }

    #[test]
    fn test_subu() {
        let mut cpu = Cpu::new();
        cpu.registers[7] = 1;
        cpu.registers[8] = 2;
        cpu.execute(Instruction(0xE84023)); // SUBU r8, r7, r8
        assert_eq!(cpu.registers[8], (-1 as i32) as u32);
    }

    #[test]
    fn test_mfhi() {
        let mut cpu = Cpu::new();
        cpu.hi = 0x1234_5678;
        cpu.execute(Instruction(0x00000010)); // MFHI r0
        assert_eq!(cpu.registers[0], 0); // ignore write to r0

        cpu.execute(Instruction(0x00000810)); // MFHI r1
        assert_eq!(cpu.registers[1], 0x1234_5678);
    }

    #[test]
    fn test_mthi() {
        let mut cpu = Cpu::new();
        cpu.registers[7] = 0x1234_5678;
        cpu.hi = 0xdead_beef;
        cpu.execute(Instruction(0x00e00011)); // MTHI r7
        assert_eq!(cpu.hi, 0x1234_5678);
    }

    #[test]
    fn test_mflo() {
        let mut cpu = Cpu::new();
        cpu.lo = 0x1234_5678;
        cpu.execute(Instruction(0x00000012)); // MFLO r0
        assert_eq!(cpu.registers[0], 0); // ignore write to r0

        cpu.execute(Instruction(0x00000812)); // MFLO r1
        assert_eq!(cpu.registers[1], 0x1234_5678);
    }

    #[test]
    fn test_mtlo() {
        let mut cpu = Cpu::new();
        cpu.registers[7] = 0x1234_5678;
        cpu.lo = 0xdead_beef;
        cpu.execute(Instruction(0x00e00013)); // MTLO r7
        assert_eq!(cpu.lo, 0x1234_5678);
    }

    #[test]
    fn test_mult() {
        let mut cpu = Cpu::new();
        
        cpu.registers[7] = 0x0000_0002;
        cpu.registers[8] = 0x0000_0003;
        cpu.execute(Instruction(0x01070018)); // MULT r7, r8
        
        assert_eq!(cpu.hi, 0);
        assert_eq!(cpu.lo, 6);

        cpu.registers[7] = 0x8000_0002; // this gets sign-extended
        cpu.registers[8] = 4;
        cpu.execute(Instruction(0x01070018)); // MULT r7, r8
        
        assert_eq!(cpu.hi, 0xffff_fffe); // the result is negative
        assert_eq!(cpu.lo, 8);
    }

    #[test]
    fn test_multu() {
        let mut cpu = Cpu::new();
        
        cpu.registers[7] = 0x0000_0002;
        cpu.registers[8] = 0x0000_0003;
        cpu.execute(Instruction(0x01070019)); // MULTU r7, r8
        
        assert_eq!(cpu.hi, 0);
        assert_eq!(cpu.lo, 6);

        cpu.registers[7] = 0x8000_0002; // this does not get sign-extended
        cpu.registers[8] = 4;
        cpu.execute(Instruction(0x01070019)); // MULTU r7, r8
        
        assert_eq!(cpu.hi, 2); // the result is positive
        assert_eq!(cpu.lo, 8);
    }
}
