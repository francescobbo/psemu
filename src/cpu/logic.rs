use super::{Cpu, Instruction};

impl Cpu {
    /// 00.00 - SLL - R-Type
    /// SLL rd, rt, shamt
    /// GPR[rd] = GPR[rt] << shamt
    pub(super) fn ins_sll(&mut self, instruction: Instruction) {
        self.write_reg(
            instruction.rd(),
            self.get_rt(instruction) << instruction.shamt(),
        );
    }

    /// 00.02 - SRL - R-Type
    /// SRL rd, rt, shamt
    /// GPR[rd] = GPR[rt] >> shamt
    pub(super) fn ins_srl(&mut self, instruction: Instruction) {
        self.write_reg(
            instruction.rd(),
            self.get_rt(instruction) >> instruction.shamt(),
        );
    }

    /// 00.03 - SRA - R-Type
    /// SRA rd, rt, shamt
    /// GPR[rd] = signed(GPR[rt]) >> shamt
    pub(super) fn ins_sra(&mut self, instruction: Instruction) {
        let value = self.get_rt(instruction) as i32;
        self.write_reg(instruction.rd(), (value >> instruction.shamt()) as u32);
    }

    /// 00.04 - SLLV - R-Type
    /// SLLV rd, rt, rs
    /// GPR[rd] = GPR[rt] << (GPR[rs] & 0x1f)
    pub(super) fn ins_sllv(&mut self, instruction: Instruction) {
        self.write_reg(
            instruction.rd(),
            self.get_rt(instruction) << (self.get_rs(instruction) & 0x1f),
        );
    }

    /// 00.06 - SRLV - R-Type
    /// SRLV rd, rt, rs
    /// GPR[rd] = GPR[rt] >> (GPR[rs] & 0x1f)
    pub(super) fn ins_srlv(&mut self, instruction: Instruction) {
        self.write_reg(
            instruction.rd(),
            self.get_rt(instruction) >> (self.get_rs(instruction) & 0x1f),
        );
    }

    /// 00.07 - SRAV - R-Type
    /// SRAV rd, rt, rs
    /// GPR[rd] = signed(GPR[rt]) >> (GPR[rs] & 0x1f)
    pub(super) fn ins_srav(&mut self, instruction: Instruction) {
        let value = self.get_rt(instruction) as i32;
        self.write_reg(
            instruction.rd(),
            (value >> (self.get_rs(instruction) & 0x1f)) as u32,
        );
    }

    /// 00.24 - AND - R-Type
    /// AND rd, rs, rt
    /// GPR[rd] = GPR[rs] & GPR[rt]
    pub(super) fn ins_and(&mut self, instruction: Instruction) {
        self.write_reg(
            instruction.rd(),
            self.get_rs(instruction) & self.get_rt(instruction),
        );
    }

    /// 00.25 - OR - R-Type
    /// OR rd, rs, rt
    /// GPR[rd] = GPR[rs] | GPR[rt]
    pub(super) fn ins_or(&mut self, instruction: Instruction) {
        self.write_reg(
            instruction.rd(),
            self.get_rs(instruction) | self.get_rt(instruction),
        );
    }

    /// 00.26 - XOR - R-Type
    /// XOR rd, rs, rt
    /// GPR[rd] = GPR[rs] ^ GPR[rt]
    pub(super) fn ins_xor(&mut self, instruction: Instruction) {
        self.write_reg(
            instruction.rd(),
            self.get_rs(instruction) ^ self.get_rt(instruction),
        );
    }

    /// 00.27 - NOR - R-Type
    /// NOR rd, rs, rt
    /// GPR[rd] = ~(GPR[rs] | GPR[rt])
    pub(super) fn ins_nor(&mut self, instruction: Instruction) {
        self.write_reg(
            instruction.rd(),
            !(self.get_rs(instruction) | self.get_rt(instruction)),
        );
    }

    /// 00.2A - SLT - R-Type
    /// SLT rd, rs, rt
    /// GPR[rd] = (signed(GPR[rs]) < signed(GPR[rt])) ? 1 : 0
    pub(super) fn ins_slt(&mut self, instruction: Instruction) {
        let rs_value = self.get_rs(instruction) as i32;
        let rt_value = self.get_rt(instruction) as i32;
        self.write_reg(instruction.rd(), if rs_value < rt_value { 1 } else { 0 });
    }

    /// 00.2B - SLTU - R-Type
    /// SLTU rd, rs, rt
    /// GPR[rd] = (GPR[rs] < GPR[rt]) ? 1 : 0
    pub(super) fn ins_sltu(&mut self, instruction: Instruction) {
        self.write_reg(
            instruction.rd(),
            if self.get_rs(instruction) < self.get_rt(instruction) {
                1
            } else {
                0
            },
        );
    }

    /// 0C - ANDI - I-type
    /// ANDI rt, rs, immediate
    /// GPR[rt] = GPR[rs] & immediate_value
    pub(super) fn ins_andi(&mut self, instr: Instruction) {
        self.write_reg(instr.rt(), self.get_rs(instr) & instr.imm16());
    }

    /// 0D - ORI - I-type
    /// ORI rt, rs, immediate
    /// GPR[rt] = GPR[rs] | immediate_value
    pub(super) fn ins_ori(&mut self, instr: Instruction) {
        self.write_reg(instr.rt(), self.get_rs(instr) | instr.imm16());
    }

    /// 0E - XORI - I-type
    /// XORI rt, rs, immediate
    /// GPR[rt] = GPR[rs] ^ immediate_value
    pub(super) fn ins_xori(&mut self, instr: Instruction) {
        self.write_reg(instr.rt(), self.get_rs(instr) ^ instr.imm16());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sll_nop() {
        let mut cpu = Cpu::new();

        // SLL r0, r0, 0 => NOP
        cpu.execute(Instruction(0x00000000));

        assert_eq!(cpu.registers[0], 0);
    }

    #[test]
    fn test_sll_zero_sa() {
        let mut cpu = Cpu::new();

        // SLL r1, r1, 0
        cpu.registers[1] = 0xf00d_beef;
        cpu.execute(Instruction(0x0001_0800));

        assert_eq!(cpu.registers[1], 0xf00d_beef);
    }

    #[test]
    fn test_sll_one_sa() {
        let mut cpu = Cpu::new();

        // SLL r1, r1, 1
        cpu.registers[1] = 0xf00d_beef;
        cpu.execute(Instruction(0x0001_0840));

        assert_eq!(cpu.registers[1], 0xe01b_7dde);
    }

    #[test]
    fn test_sll_30_sa() {
        let mut cpu = Cpu::new();

        // SLL r1, r1, 30
        cpu.registers[1] = 0xf00d_beef;
        cpu.execute(Instruction(0x0001_0f80));

        assert_eq!(cpu.registers[1], 0xc000_0000);
    }

    #[test]
    fn test_sll_31_sa() {
        let mut cpu = Cpu::new();

        // SLL r1, r1, 31
        cpu.registers[1] = 0xf00d_beef;
        cpu.execute(Instruction(0x0001_0fc0));

        assert_eq!(cpu.registers[1], 0x8000_0000);
    }

    #[test]
    fn test_sll_different_regs_sa() {
        let mut cpu = Cpu::new();

        // SLL r1, r2, 6
        cpu.registers[1] = 0xf00d_beef;
        cpu.registers[2] = 0x1337_c0d3;
        cpu.execute(Instruction(0x0002_0980));

        assert_eq!(cpu.registers[1], 0xcdf0_34c0);
        assert_eq!(cpu.registers[2], 0x1337_c0d3);
    }

    #[test]
    fn test_srl_zero_sa() {
        let mut cpu = Cpu::new();

        // SRL r1, r1, 0
        cpu.registers[1] = 0xf00d_beef;
        cpu.execute(Instruction(0x0001_0802));

        assert_eq!(cpu.registers[1], 0xf00d_beef);
    }

    #[test]
    fn test_srl_one_sa() {
        let mut cpu = Cpu::new();

        // SRL r1, r1, 1
        cpu.registers[1] = 0xf00d_beef;
        cpu.execute(Instruction(0x0001_0842));

        assert_eq!(cpu.registers[1], 0x7806_df77);
    }

    #[test]
    fn test_srl_30_sa() {
        let mut cpu = Cpu::new();

        // SRL r1, r1, 30
        cpu.registers[1] = 0xf00d_beef;
        cpu.execute(Instruction(0x0001_0f82));

        assert_eq!(cpu.registers[1], 0x0000_0003);
    }

    #[test]
    fn test_srl_31_sa() {
        let mut cpu = Cpu::new();

        // SRL r1, r1, 31
        cpu.registers[1] = 0xf00d_beef;
        cpu.execute(Instruction(0x0001_0fc2));

        assert_eq!(cpu.registers[1], 0x0000_0001);
    }

    #[test]
    fn test_srl_different_regs_sa() {
        let mut cpu = Cpu::new();

        // SRL r1, r2, 6
        cpu.registers[1] = 0xf00d_beef;
        cpu.registers[2] = 0x1337_c0d3;
        cpu.execute(Instruction(0x0002_0982));

        assert_eq!(cpu.registers[1], 0x004c_df03);
        assert_eq!(cpu.registers[2], 0x1337_c0d3);
    }

    #[test]
    fn test_sra_zero_sa() {
        let mut cpu = Cpu::new();

        // SRA r1, r1, 0
        cpu.registers[1] = 0xf00d_beef;
        cpu.execute(Instruction(0x0001_0803));

        assert_eq!(cpu.registers[1], 0xf00d_beef);
    }

    #[test]
    fn test_sra_one_sa() {
        let mut cpu = Cpu::new();

        // SRA r1, r1, 1
        cpu.registers[1] = 0xf00d_beef;
        cpu.execute(Instruction(0x0001_0843));

        assert_eq!(cpu.registers[1], 0xf806_df77);
    }

    #[test]
    fn test_sra_30_sa() {
        let mut cpu = Cpu::new();

        // SRA r1, r1, 30
        cpu.registers[1] = 0xf00d_beef;
        cpu.execute(Instruction(0x0001_0f83));

        assert_eq!(cpu.registers[1], 0xffff_ffff);
    }

    #[test]
    fn test_sra_31_sa() {
        let mut cpu = Cpu::new();

        // SRA r1, r1, 31
        cpu.registers[1] = 0xf00d_beef;
        cpu.execute(Instruction(0x0001_0fc3));

        assert_eq!(cpu.registers[1], 0xffff_ffff);
    }

    #[test]
    fn test_sra_different_regs_sa() {
        let mut cpu = Cpu::new();

        // SRA r1, r2, 6
        cpu.registers[1] = 0xf00d_beef;
        cpu.registers[2] = 0x1337_c0d3;
        cpu.execute(Instruction(0x0002_0983));

        assert_eq!(cpu.registers[1], 0x004c_df03);
        assert_eq!(cpu.registers[2], 0x1337_c0d3);
    }

    #[test]
    fn test_andi() {
        let mut cpu = Cpu::new();
        cpu.registers[7] = 0b1100_1101;
        cpu.execute(Instruction(0x30e8_00ff)); // ANDI r8, r7, 0xff
        assert_eq!(cpu.registers[8], 0b1100_1101);

        cpu.execute(Instruction(0x30e8_00f0)); // ANDI r8, r7, 0xf0
        assert_eq!(cpu.registers[8], 0b1100_0000);

        // test that the upper 16 bits are always lost
        cpu.registers[7] = 0xffff_ffff;
        cpu.execute(Instruction(0x30e8_ffff)); // ANDI r8, r7, 0xffff
        assert_eq!(cpu.registers[8], 0xffff);
    }

    #[test]
    fn test_ori() {
        let mut cpu = Cpu::new();
        cpu.registers[7] = 0b1100_1101;
        cpu.execute(Instruction(0x34e8_00ff)); // ORI r8, r7, 0xff
        assert_eq!(cpu.registers[8], 0b1111_1111);

        cpu.execute(Instruction(0x34e8_00f0)); // ORI r8, r7, 0xf0
        assert_eq!(cpu.registers[8], 0b1111_1101);
    }

    #[test]
    fn test_xori() {
        let mut cpu = Cpu::new();
        cpu.registers[7] = 0b1100_1101;
        cpu.execute(Instruction(0x38e8_00ff)); // XORI r8, r7, 0xff
        assert_eq!(cpu.registers[8], 0b0011_0010);

        cpu.execute(Instruction(0x38e8_00f0)); // XORI r8, r7, 0xf0
        assert_eq!(cpu.registers[8], 0b0011_1101);
    }

    #[test]
    fn test_sllv_basic() {
        let mut cpu = Cpu::new();

        // SLLV r3, r2, r1
        cpu.registers[1] = 4;
        cpu.registers[2] = 0x0000_000f;
        cpu.execute(Instruction(0x00221804));

        assert_eq!(cpu.registers[3], 0x0000_00f0);
        assert_eq!(cpu.registers[1], 4);
        assert_eq!(cpu.registers[2], 0x0000_000f);
    }

    #[test]
    fn test_sllv_shift_by_zero() {
        let mut cpu = Cpu::new();

        // SLLV r3, r2, r1
        cpu.registers[1] = 0;
        cpu.registers[2] = 0xabcd_1234;
        cpu.execute(Instruction(0x00221804));

        assert_eq!(cpu.registers[3], 0xabcd_1234);
    }

    #[test]
    fn test_sllv_shift_by_31() {
        let mut cpu = Cpu::new();

        // SLLV r3, r2, r1
        cpu.registers[1] = 31;
        cpu.registers[2] = 0x0000_0001;
        cpu.execute(Instruction(0x00221804));

        assert_eq!(cpu.registers[3], 0x8000_0000);
    }

    #[test]
    fn test_sllv_shift_amount_masked() {
        let mut cpu = Cpu::new();

        // SLLV r3, r2, r1
        cpu.registers[1] = 0x22;
        cpu.registers[2] = 0x1;
        cpu.execute(Instruction(0x00221804));

        // The shift amount is masked to 0x1f
        assert_eq!(cpu.registers[3], 0x4); // 1 << 2
    }

    #[test]
    fn test_srlv_basic() {
        let mut cpu = Cpu::new();

        // SRLV r3, r2, r1
        cpu.registers[1] = 4;
        cpu.registers[2] = 0xf000_0000;
        cpu.execute(Instruction(0x00221806));

        assert_eq!(cpu.registers[3], 0x0f00_0000);
    }

    #[test]
    fn test_srlv_shift_by_31() {
        let mut cpu = Cpu::new();

        // SRLV r3, r2, r1
        cpu.registers[1] = 31;
        cpu.registers[2] = 0xf00d_beef;
        cpu.execute(Instruction(0x00221806));

        assert_eq!(cpu.registers[3], 0x0000_0001); // (0xf00d_beef >> 31)
    }

    #[test]
    fn test_srav_basic_positive() {
        let mut cpu = Cpu::new();

        cpu.registers[1] = 4;
        cpu.registers[2] = 0x0f00_0000;
        cpu.execute(Instruction(0x00221807));

        assert_eq!(cpu.registers[3], 0x00f0_0000);
    }

    #[test]
    fn test_srav_basic_negative() {
        let mut cpu = Cpu::new();

        // SRAV r3, r2, r1
        cpu.registers[1] = 4;
        cpu.registers[2] = 0xf000_0000;
        cpu.execute(Instruction(0x00221807));

        assert_eq!(cpu.registers[3], 0xff00_0000); // Sign bit propagated
    }

    #[test]
    fn test_srav_shift_by_31_negative() {
        let mut cpu = Cpu::new();

        // SRAV r3, r2, r1
        cpu.registers[1] = 31;
        cpu.registers[2] = 0xf00d_beef;
        cpu.execute(Instruction(0x00221807));

        assert_eq!(cpu.registers[3], 0xffff_ffff);
    }

    // AND Tests
    // funct = 0x24
    // AND rd, rs, rt
    #[test]
    fn test_and_basic() {
        let mut cpu = Cpu::new();
        // AND r3, r1, r2
        cpu.registers[1] = 0xffff_0000;
        cpu.registers[2] = 0x00ff_ff00;
        // rs=1, rt=2, rd=3, shamt_field=0, funct=0x24
        cpu.execute(Instruction(0x00221824));
        assert_eq!(cpu.registers[3], 0x00ff_0000);
    }

    // OR Tests
    // funct = 0x25
    // OR rd, rs, rt
    #[test]
    fn test_or_basic() {
        let mut cpu = Cpu::new();
        // OR r3, r1, r2
        cpu.registers[1] = 0xF0F0_0000;
        cpu.registers[2] = 0x0F0F_0F0F;
        cpu.execute(Instruction(0x00221825));
        assert_eq!(cpu.registers[3], 0xffff0f0f);
    }

    #[test]
    fn test_xor_basic() {
        let mut cpu = Cpu::new();

        // XOR r3, r1, r2
        cpu.registers[1] = 0xffff_0000;
        cpu.registers[2] = 0x00ff_ff00;
        cpu.execute(Instruction(0x00221826));

        assert_eq!(cpu.registers[3], 0xff00_ff00);
    }

    #[test]
    fn test_nor_basic() {
        let mut cpu = Cpu::new();

        cpu.registers[1] = 0xf000_0000; // 11110000...
        cpu.registers[2] = 0x0000_000f; // 00000000...00001111
        // OR result: 0xf000000f (11110000...00001111)
        // NOR result:0x0ffffff0 (00001111...11110000)
        cpu.execute(Instruction(0x00221827));

        assert_eq!(cpu.registers[3], 0x0fff_fff0);
    }

    #[test]
    fn test_nor_both_zero() {
        let mut cpu = Cpu::new();

        // NOR r3, r1, r2
        cpu.registers[1] = 0x0000_0000;
        cpu.registers[2] = 0x0000_0000;
        cpu.execute(Instruction(0x00221827));

        assert_eq!(cpu.registers[3], 0xffff_ffff);
    }

    #[test]
    fn test_slt_less_than_positive() {
        let mut cpu = Cpu::new();

        // SLT r3, r1, r2
        cpu.registers[1] = 10;
        cpu.registers[2] = 20;
        cpu.execute(Instruction(0x0022182a));

        assert_eq!(cpu.registers[3], 1);
    }

    #[test]
    fn test_slt_greater_than_positive() {
        let mut cpu = Cpu::new();

        // SLT r3, r1, r2
        cpu.registers[1] = 20;
        cpu.registers[2] = 10;
        cpu.execute(Instruction(0x0022182a));

        assert_eq!(cpu.registers[3], 0);
    }

    #[test]
    fn test_slt_equal_positive() {
        let mut cpu = Cpu::new();

        // SLT r3, r1, r2
        cpu.registers[1] = 10;
        cpu.registers[2] = 10;
        cpu.execute(Instruction(0x0022182a));

        assert_eq!(cpu.registers[3], 0);
    }

    #[test]
    fn test_slt_negative_less_than_positive() {
        let mut cpu = Cpu::new();

        // SLT r3, r1, r2
        cpu.registers[1] = -5i32 as u32;
        cpu.registers[2] = 5;
        cpu.execute(Instruction(0x0022182a));

        assert_eq!(cpu.registers[3], 1);
    }

    #[test]
    fn test_slt_negative_less_than_negative() {
        let mut cpu = Cpu::new();

        // SLT r3, r1, r2
        cpu.registers[1] = -10i32 as u32;
        cpu.registers[2] = -5i32 as u32;
        cpu.execute(Instruction(0x0022182a));

        assert_eq!(cpu.registers[3], 1);
    }

    #[test]
    fn test_slt_positive_greater_than_negative() {
        let mut cpu = Cpu::new();

        // SLT r3, r1, r2
        cpu.registers[1] = 5;
        cpu.registers[2] = -5i32 as u32;
        cpu.execute(Instruction(0x0022182a));

        assert_eq!(cpu.registers[3], 0);
    }

    #[test]
    fn test_sltu_less_than() {
        let mut cpu = Cpu::new();

        // SLTU r3, r1, r2
        cpu.registers[1] = 10;
        cpu.registers[2] = 20;
        cpu.execute(Instruction(0x0022182b));

        assert_eq!(cpu.registers[3], 1);
    }

    #[test]
    fn test_sltu_greater_than() {
        let mut cpu = Cpu::new();

        // SLTU r3, r1, r2
        cpu.registers[1] = 20;
        cpu.registers[2] = 10;
        cpu.execute(Instruction(0x0022182b));

        assert_eq!(cpu.registers[3], 0);
    }

    #[test]
    fn test_sltu_equal() {
        let mut cpu = Cpu::new();

        // SLTU r3, r1, r2
        cpu.registers[1] = 10;
        cpu.registers[2] = 10;
        cpu.execute(Instruction(0x0022182b));

        assert_eq!(cpu.registers[3], 0);
    }

    #[test]
    fn test_sltu_negative_vs_positive() {
        let mut cpu = Cpu::new();

        // SLTU r3, r1, r2
        cpu.registers[1] = -5i32 as u32;
        cpu.registers[2] = 5;
        cpu.execute(Instruction(0x0022182b));

        assert_eq!(cpu.registers[3], 0);
    }

    #[test]
    fn test_sltu_max_unsigned_vs_zero() {
        let mut cpu = Cpu::new();

        // SLTU r3, r1, r2
        cpu.registers[1] = 0xffffffff;
        cpu.registers[2] = 0;
        cpu.execute(Instruction(0x0022182b));

        assert_eq!(cpu.registers[3], 0);
    }

    #[test]
    fn test_sltu_zero_vs_max_unsigned() {
        let mut cpu = Cpu::new();

        // SLTU r3, r1, r2
        cpu.registers[1] = 0;
        cpu.registers[2] = 0xffffffff;
        cpu.execute(Instruction(0x0022182b));
        assert_eq!(cpu.registers[3], 1);
    }
}
