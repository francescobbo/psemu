use super::{Cpu, Instruction};

impl Cpu {
    /// 00.10 - MFHI - R-Type
    /// MFHI rd
    /// GPR[rd] = HI
    pub(super) fn ins_mfhi(&mut self, instruction: Instruction) {
        self.write_reg(instruction.rd(), self.hi);
    }

    /// 00.11 - MTHI - R-Type
    /// MTHI rs
    /// HI = GPR[rs]
    pub(super) fn ins_mthi(&mut self, instruction: Instruction) {
        self.hi = self.get_rs(instruction);
    }

    /// 00.12 - MFLO - R-Type
    /// MFLO rd
    /// GPR[rd] = LO
    pub(super) fn ins_mflo(&mut self, instruction: Instruction) {
        self.write_reg(instruction.rd(), self.lo);
    }

    /// 00.13 - MTLO - R-Type
    /// MTLO rs
    /// LO = GPR[rs]
    pub(super) fn ins_mtlo(&mut self, instruction: Instruction) {
        self.lo = self.get_rs(instruction);
    }

    /// 00.18 - MULT - R-Type
    /// MULT rs, rt
    /// result = sign_extended64(GPR[rs]) * sign_extended64(GPR[rt])
    /// HI = result[63:32]
    /// LO = result[31:0]
    pub(super) fn ins_mult(&mut self, instruction: Instruction) {
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
    pub(super) fn ins_multu(&mut self, instruction: Instruction) {
        let rs = self.get_rs(instruction) as u64;
        let rt = self.get_rt(instruction) as u64;
        let result = rs.wrapping_mul(rt);

        self.hi = (result >> 32) as u32;
        self.lo = result as u32;
    }

    /// 00.1A - DIV - R-Type
    /// DIV rs, rt
    /// HI = GPR[rs] % GPR[rt]
    /// LO = GPR[rs] / GPR[rt]
    pub(super) fn ins_div(&mut self, instruction: Instruction) {
        let dividend = self.get_rs(instruction) as i32;
        let divisor = self.get_rt(instruction) as i32;

        if divisor == 0 {
            self.hi = dividend as u32;
            if dividend >= 0 {
                self.lo = 0xffff_ffff;
            } else {
                self.lo = 1;
            }
        } else if dividend as u32 == 0x8000_0000 && divisor == -1 {
            self.hi = 0;
            self.lo = 0x8000_0000;
        } else {
            self.hi = (dividend % divisor) as u32;
            self.lo = (dividend / divisor) as u32;
        }
    }

    /// 00.1B - DIVU - R-Type
    /// DIVU rs, rt
    /// HI = GPR[rs] % GPR[rt]
    /// LO = GPR[rs] / GPR[rt]
    pub(super) fn ins_divu(&mut self, instruction: Instruction) {
        let dividend = self.get_rs(instruction);
        let divisor = self.get_rt(instruction);

        if divisor == 0 {
            self.hi = dividend;
            self.lo = 0xffff_ffff;
        } else {
            self.hi = dividend % divisor;
            self.lo = dividend / divisor;
        }
    }

    /// 00.20 - ADD - R-Type
    /// ADD rd, rs, rt
    /// GPR[rd] = GPR[rs] + GPR[rt]
    ///
    /// Causes overflow exception if the result is not representable in 32 bits
    pub(super) fn ins_add(&mut self, instruction: Instruction) {
        let rs = self.get_rs(instruction) as i32;
        let rt = self.get_rt(instruction) as i32;

        self.write_reg(instruction.rd(), (rs + rt) as u32);
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

    /// 00.22 - SUB - R-Type
    /// SUB rd, rs, rt
    /// GPR[rd] = GPR[rs] - GPR[rt]
    ///
    /// Causes overflow exception if the result is not representable in 32 bits
    pub(super) fn ins_sub(&mut self, instruction: Instruction) {
        let rs = self.get_rs(instruction) as i32;
        let rt = self.get_rt(instruction) as i32;

        self.write_reg(instruction.rd(), (rs - rt) as u32);
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

    /// 08 - ADDI - I-type
    /// ADDIU rt, rs, immediate
    /// GPR[rt] = GPR[rs] + sign_extended(immediate_value)
    ///
    /// Causes overflow exception if the result is not representable in 32 bits
    pub(super) fn ins_addi(&mut self, instr: Instruction) {
        let value = self.get_rs(instr) as i32;
        let immediate = instr.simm16();
        
        match value.checked_add(immediate) {
            Some(result) => self.write_reg(instr.rt(), result as u32),
            None => self.exception("Overflow")
        }
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
    use crate::cpu::test_utils::*;

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

    #[test]
    fn test_div() {
        let mut cpu = Cpu::new();

        cpu.registers[7] = 0x0000_0002;
        cpu.registers[8] = 0x0000_0003;
        cpu.execute(Instruction(0x0107001A)); // DIV r8, r7

        assert_eq!(cpu.hi, 1);
        assert_eq!(cpu.lo, 1);

        cpu.registers[7] = -1i32 as u32;
        cpu.registers[8] = 0x8000_0002; // this gets sign-extended
        cpu.execute(Instruction(0x0107001A)); // DIV r8, r7

        assert_eq!(cpu.hi, 0);
        assert_eq!(cpu.lo, 0x7fff_fffe);

        cpu.registers[7] = 0;
        cpu.registers[8] = 5;
        cpu.execute(Instruction(0x0107001A)); // DIV r8, r7

        assert_eq!(cpu.hi, 5);
        assert_eq!(cpu.lo, 0xffff_ffff);

        cpu.registers[7] = 0;
        cpu.registers[8] = -5i32 as u32;
        cpu.execute(Instruction(0x0107001A)); // DIV r8, r7

        assert_eq!(cpu.hi, -5i32 as u32);
        assert_eq!(cpu.lo, 1);

        cpu.registers[7] = -1i32 as u32;
        cpu.registers[8] = 0x8000_0000; // largest negative number
        cpu.execute(Instruction(0x0107001A)); // DIV r8, r7

        assert_eq!(cpu.hi, 0);
        assert_eq!(cpu.lo, 0x8000_0000);
    }

    #[test]
    fn test_divu() {
        let mut cpu = Cpu::new();

        cpu.registers[7] = 0x0000_0002;
        cpu.registers[8] = 0x0000_0003;
        cpu.execute(Instruction(0x0107001B)); // DIVU r8, r7

        assert_eq!(cpu.hi, 1);
        assert_eq!(cpu.lo, 1);

        cpu.registers[7] = -1i32 as u32;
        cpu.registers[8] = 0x8000_0002; // this does not get sign-extended
        cpu.execute(Instruction(0x0107001B)); // DIVU r8, r7

        assert_eq!(cpu.hi, 0x80000002);
        assert_eq!(cpu.lo, 0);

        cpu.registers[7] = 0;
        cpu.registers[8] = 5;
        cpu.execute(Instruction(0x0107001B)); // DIVU r8, r7

        assert_eq!(cpu.hi, 5);
        assert_eq!(cpu.lo, 0xffff_ffff);
    }

    #[test]
    fn test_add() {
        let mut cpu = test_cpu(
            &[(1, 1234), (2, 0xffffffff), (3, 15), (7, 1)],
            &[
                // ADD r8, r7, r0
                r_type(0x20, 8, 0, 7),
                // ADD r9, r7, r1
                r_type(0x20, 9, 1, 7),
                // ADD r10, r7, r2
                r_type(0x20, 10, 2, 7),
                // ADD r0, r7, r3
                r_type(0x20, 0, 3, 7),
            ],
        );
        cpu_steps(&mut cpu, 4);

        assert_eq!(cpu.registers[8], 1);
        assert_eq!(cpu.registers[9], 1 + 1234);
        assert_eq!(cpu.registers[10], 0); // subtraction does not overflow
        assert_eq!(cpu.registers[0], 0);
    }

    #[test]
    #[should_panic]
    fn test_add_overflow() {
        let mut cpu = test_cpu(
            &[(7, 0x7fff_ffff), (1, 1)],
            &[
                // ADD r8, r7, r1
                r_type(0x20, 8, 1, 7),
            ],
        );
        cpu.step();

        // This should panic due to overflow as 7fff_ffff is the largest signed
        // 32-bit integer, and adding 1 would take us to the largest signed
        // negative integer.
    }

    #[test]
    fn test_addu() {
        let mut cpu = test_cpu(
            &[
                (1, 1234),
                (2, 0xffffffff),
                (3, 15),
                (7, 1)],
            &[
                // ADDU r8, r7, r0
                r_type(0x21, 8, 0, 7),
                // ADDU r9, r7, r1
                r_type(0x21, 9, 1, 7),
                // ADDU r10, r7, r2
                r_type(0x21, 10,2, 7),
                // ADDU r0, r7, r3
                r_type(0x21, 0, 3, 7),
                // ADDU r11, r2, r2
                r_type(0x21, 11, 2, 2),
            ],
        );
        cpu_steps(&mut cpu, 5);

        assert_eq!(cpu.registers[8], 1);
        assert_eq!(cpu.registers[9], 1 + 1234);
        assert_eq!(cpu.registers[10], 0); // subtraction does not overflow
        assert_eq!(cpu.registers[0], 0);
        assert_eq!(cpu.registers[11], 0xfffffffe); // wrapping addition
    }

    #[test]
    fn test_sub() {
        let mut cpu = test_cpu(
            &[(1, 1234), (2, 0xffffffff), (3, 15), (7, 1)],
            &[
                // SUB r8, r7, r0
                r_type(0x22, 8, 0, 7),
                // SUB r9, r7, r1
                r_type(0x22, 9, 1, 7),
                // SUB r10, r7, r2
                r_type(0x22, 10, 2, 7),
                // SUB r0, r7, r3
                r_type(0x22, 0, 3, 7),
            ],
        );
        cpu_steps(&mut cpu, 4);

        assert_eq!(cpu.registers[8], 1);
        assert_eq!(cpu.registers[9], (1 - 1234) as u32);
        assert_eq!(cpu.registers[10], 2); // addition does not overflow
        assert_eq!(cpu.registers[0], 0);
    }

    #[test]
    #[should_panic]
    fn test_sub_overflow() {
        let mut cpu = test_cpu(
            &[(7, 0x8000_0000), (1, 1)],
            &[
                // SUB r8, r7, r1
                r_type(0x22, 8, 1, 7),
            ],
        );
        cpu.step();

        // This should panic due to overflow as 8000_0000 is the biggest signed
        // negative 32-bit integer, and subtracting 1 would take us to the
        // largest signed positive integer.
    }

    #[test]
    fn test_subu() {
        let mut cpu = test_cpu(
            &[(7, 1), (8, 2)],
            &[
                // SUBU r9, r7, r8
                r_type(0x23, 9, 8, 7),
            ],
        );

        cpu.step();
        assert_eq!(cpu.registers[9], (-1 as i32) as u32);
    }

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
                i_type(0x08, 10, 7, 0xffff),
                // ADDI r0, r7, 15
                i_type(0x08, 0, 7, 15),
            ],
        );
        cpu_steps(&mut cpu, 4);

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
