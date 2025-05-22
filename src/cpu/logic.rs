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

    /// 0A - SLTI - I-type
    /// SLTI rt, rs, immediate
    /// GPR[rt] = (GPR[rs] < sign_extended(immediate_value)) ? 1 : 0
    pub fn ins_slti(&mut self, instruction: Instruction) {
        let value = instruction.simm16();
        self.write_reg(
            instruction.rt(),
            if (self.get_rs(instruction) as i32) < value {
                1
            } else {
                0
            },
        )
    }

    /// 0B - SLTIU - I-type
    /// SLTIU rt, rs, immediate
    /// GPR[rt] = (GPR[rs] < zero_extended(immediate_value)) ? 1 : 0
    pub fn ins_sltiu(&mut self, instruction: Instruction) {
        let value = instruction.simm16() as u32;
        self.write_reg(
            instruction.rt(),
            if self.get_rs(instruction) < value {
                1
            } else {
                0
            },
        )
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
    use crate::cpu::test_utils::*;

    #[test]
    fn test_sll() {
        let mut cpu = test_cpu(
            &[(1, 0xf00d_beef)],
            &[
                // NOP (SLL r0, r0, 0)
                r_type_shift(0, 0, 0, 0),
                // SLL r1, r1, 0
                r_type_shift(0, 1, 1, 0),
                // SLL r2, r1, 1
                r_type_shift(0, 2, 1, 1),
                // SLL r3, r1, 30
                r_type_shift(0, 3, 1, 30),
                // SLL r4, r1, 31
                r_type_shift(0, 4, 1, 31),
            ],
        );

        cpu_steps(&mut cpu, 5);

        assert_eq!(cpu.registers[0], 0);
        assert_eq!(cpu.registers[1], 0xf00d_beef); // unchanged
        assert_eq!(cpu.registers[2], 0xe01b_7dde); // << 1
        assert_eq!(cpu.registers[3], 0xc000_0000); // << 30
        assert_eq!(cpu.registers[4], 0x8000_0000); // << 31
    }

    #[test]
    fn test_srl() {
        let mut cpu = test_cpu(
            &[(1, 0xf00d_beef)],
            &[
                // SRL r1, r1, 0
                r_type_shift(2, 1, 1, 0),
                // SRL r2, r1, 1
                r_type_shift(2, 2, 1, 1),
                // SRL r3, r1, 30
                r_type_shift(2, 3, 1, 30),
                // SRL r4, r1, 31
                r_type_shift(2, 4, 1, 31),
            ],
        );

        cpu_steps(&mut cpu, 4);

        assert_eq!(cpu.registers[1], 0xf00d_beef);
        assert_eq!(cpu.registers[2], 0x7806_df77);
        assert_eq!(cpu.registers[3], 0x0000_0003);
        assert_eq!(cpu.registers[4], 0x0000_0001);
    }

    #[test]
    fn test_sra() {
        let mut cpu = test_cpu(
            &[(1, 0xf00d_beef)],
            &[
                // SRA r1, r1, 0
                r_type_shift(3, 1, 1, 0),
                // SRA r2, r1, 1
                r_type_shift(3, 2, 1, 1),
                // SRA r3, r1, 30
                r_type_shift(3, 3, 1, 30),
                // SRA r4, r1, 31
                r_type_shift(3, 4, 1, 31),
            ],
        );

        cpu_steps(&mut cpu, 4);

        assert_eq!(cpu.registers[1], 0xf00d_beef);
        assert_eq!(cpu.registers[2], 0xf806_df77);
        assert_eq!(cpu.registers[3], 0xffff_ffff);
        assert_eq!(cpu.registers[4], 0xffff_ffff);
    }

    #[test]
    fn test_sllv() {
        let mut cpu = test_cpu(
            &[
                (1, 4),
                (2, 0x0000_000f),
                (4, 0),
                (5, 0xabcd_1234),
                (7, 31),
                (8, 1),
                (10, 0x22),
                (11, 1),
            ],
            &[
                // SLLV r3, r2, r1
                r_type(0x04, 3, 2, 1),
                // SLLV r6, r5, r4
                r_type(0x04, 6, 5, 4),
                // SLLV r9, r8, r7
                r_type(0x04, 9, 8, 7),
                // SLLV r12, r11, r10
                r_type(0x04, 12, 11, 10),
            ],
        );

        cpu_steps(&mut cpu, 4);

        assert_eq!(cpu.registers[1], 4);
        assert_eq!(cpu.registers[2], 0x0000_000f);
        assert_eq!(cpu.registers[3], 0x0000_00f0);

        assert_eq!(cpu.registers[4], 0);
        assert_eq!(cpu.registers[5], 0xabcd_1234);
        assert_eq!(cpu.registers[6], 0xabcd_1234); // No shift

        assert_eq!(cpu.registers[7], 31);
        assert_eq!(cpu.registers[8], 1);
        assert_eq!(cpu.registers[9], 0x8000_0000); // 1 << 31

        assert_eq!(cpu.registers[10], 0x22);
        assert_eq!(cpu.registers[11], 1);
        assert_eq!(cpu.registers[12], 0x4); // 1 << (0x22 & 0x1f)
    }

    #[test]
    fn test_srlv() {
        let mut cpu = test_cpu(
            &[(1, 4), (2, 0xf000_0000), (4, 31), (5, 0xf00d_beef)],
            &[
                // SRLV r3, r2, r1
                r_type(0x06, 3, 2, 1),
                // SRLV r6, r5, r4
                r_type(0x06, 6, 5, 4),
            ],
        );

        cpu_steps(&mut cpu, 4);

        assert_eq!(cpu.registers[1], 4);
        assert_eq!(cpu.registers[2], 0xf000_0000);
        assert_eq!(cpu.registers[3], 0x0f00_0000);

        assert_eq!(cpu.registers[4], 31);
        assert_eq!(cpu.registers[5], 0xf00d_beef);
        assert_eq!(cpu.registers[6], 1); // 0xf00d_beef >> 31
    }

    #[test]
    fn test_srav() {
        let mut cpu = test_cpu(
            &[
                (1, 4),
                (2, 0xf000_0000),
                (4, 31),
                (5, 0xf00d_beef),
                (7, 1),
                (8, 0x700d_beef),
            ],
            &[
                // SRAV r3, r2, r1
                r_type(0x07, 3, 2, 1),
                // SRAV r6, r5, r4
                r_type(0x07, 6, 5, 4),
                // SRAV r9, r8, r7
                r_type(0x07, 9, 8, 7),
            ],
        );

        cpu_steps(&mut cpu, 3);

        assert_eq!(cpu.registers[1], 4);
        assert_eq!(cpu.registers[2], 0xf000_0000);
        assert_eq!(cpu.registers[3], 0xff00_0000); // (signed) 0xf000_0000 >> 4

        assert_eq!(cpu.registers[4], 31);
        assert_eq!(cpu.registers[5], 0xf00d_beef);
        assert_eq!(cpu.registers[6], 0xffffffff); // (signed) 0xf00d_beef >> 31

        assert_eq!(cpu.registers[7], 1);
        assert_eq!(cpu.registers[8], 0x700d_beef);
        assert_eq!(cpu.registers[9], 0x3806_df77); // (signed) 0x700d_beef >> 1
    }

    #[test]
    fn test_and() {
        let mut cpu = test_cpu(
            &[(1, 0xffff_0000), (2, 0x00ff_ff00)],
            &[
                // AND r3, r1, r2
                r_type(0x24, 3, 1, 2),
            ],
        );

        cpu.step();
        assert_eq!(cpu.registers[3], 0x00ff_0000);
    }

    #[test]
    fn test_or() {
        let mut cpu = test_cpu(
            &[(1, 0xf0f0_0000), (2, 0x0f0f_0f0f)],
            &[
                // OR r3, r1, r2
                r_type(0x25, 3, 1, 2),
            ],
        );

        cpu.step();
        assert_eq!(cpu.registers[3], 0xffff0f0f);
    }

    #[test]
    fn test_xor() {
        let mut cpu = test_cpu(
            &[(1, 0xffff_0000), (2, 0x00ff_ff00)],
            &[
                // XOR r3, r1, r2
                r_type(0x26, 3, 1, 2),
            ],
        );

        cpu.step();
        assert_eq!(cpu.registers[3], 0xff00_ff00);
    }

    #[test]
    fn test_nor() {
        let mut cpu = test_cpu(
            &[(1, 0xf000_0000), (2, 0x0000_000f)],
            &[
                // NOR r3, r1, r2
                r_type(0x27, 3, 1, 2),
                // NOR r4, r0, r0
                r_type(0x27, 4, 0, 0),
            ],
        );

        cpu_steps(&mut cpu, 2);

        assert_eq!(cpu.registers[3], 0x0fff_fff0);
        assert_eq!(cpu.registers[4], 0xffff_ffff);
    }

    #[test]
    fn test_slt() {
        let mut cpu = test_cpu(
            &[(1, 10), (2, 20), (3, 0xffff_fffb), (4, 5), (5, 0xffff_fff0)],
            &[
                // SLT r10, r1, r2
                r_type(0x2a, 10, 2, 1),
                // SLT r11, r2, r1
                r_type(0x2a, 11, 1, 2),
                // SLT r12, r1, r1
                r_type(0x2a, 12, 1, 1),
                // SLT r13, r3, r4
                r_type(0x2a, 13, 4, 3),
                // SLT r14, r4, r3
                r_type(0x2a, 14, 3, 4),
                // SLT r15, r5, r3
                r_type(0x2a, 15, 3, 5),
            ],
        );

        cpu_steps(&mut cpu, 6);

        assert_eq!(cpu.registers[10], 1); // 10 < 20
        assert_eq!(cpu.registers[11], 0); // 20 < 10
        assert_eq!(cpu.registers[12], 0); // 10 < 10
        assert_eq!(cpu.registers[13], 1); // -5 < 5
        assert_eq!(cpu.registers[14], 0); // 5 < -5
        assert_eq!(cpu.registers[15], 1); // -5 < -16
    }

    #[test]
    fn test_sltu() {
        let mut cpu = test_cpu(
            &[(1, 10), (2, 20), (3, 0xffff_fffb), (4, 5), (5, 0xffffffff)],
            &[
                // SLTU r10, r1, r2
                r_type(0x2b, 10, 2, 1),
                // SLTU r11, r2, r1
                r_type(0x2b, 11, 1, 2),
                // SLTU r12, r1, r1
                r_type(0x2b, 12, 1, 1),
                // SLTU r13, r3, r4
                r_type(0x2b, 13, 4, 3),
                // SLTU r14, r4, r3
                r_type(0x2b, 14, 3, 4),
                // SLTU r15, r5, r3
                r_type(0x2b, 15, 3, 5),
                // SLTU r16, r5, r0
                r_type(0x2b, 16, 0, 5),
            ],
        );

        cpu_steps(&mut cpu, 7);

        assert_eq!(cpu.registers[10], 1); // 10 < 20
        assert_eq!(cpu.registers[11], 0); // 20 < 10
        assert_eq!(cpu.registers[12], 0); // 10 < 10
        assert_eq!(cpu.registers[13], 0); // -5 (unsigned) < 5
        assert_eq!(cpu.registers[14], 1); // 5 (unsigned) < -5 (unsigned)
        assert_eq!(cpu.registers[15], 0); // -5 (unsigned) < -1 (unsigned)
        assert_eq!(cpu.registers[16], 0); // -1 (unsigned) < 0
    }

    #[test]
    fn test_slti() {
        let mut cpu = test_cpu(
            &[(7, 1)],
            &[
                // SLTI r8, r7, 0
                i_type(0x0a, 8, 7, 1),
                // SLTI r8, r7, 1234
                i_type(0x0a, 8, 7, 0),
                // SLTI r8, r7, -1
                i_type(0x0a, 8, 7, 0xffff),
                // SLTI r8, r7, 2
                i_type(0x0a, 8, 7, 2),
            ],
        );

        cpu.step();
        assert_eq!(cpu.registers[8], 0);
        cpu.step();
        assert_eq!(cpu.registers[8], 0);
        cpu.step();
        assert_eq!(cpu.registers[8], 0);
        cpu.step();
        assert_eq!(cpu.registers[8], 1);
    }

    #[test]
    fn test_sltiu() {
        let mut cpu = test_cpu(
            &[(7, 1)],
            &[
                // SLTIU r8, r7, 0
                i_type(0x0b, 8, 7, 1),
                // SLTIU r8, r7, 1234
                i_type(0x0b, 8, 7, 0),
                // SLTIU r8, r7, -1
                i_type(0x0b, 8, 7, 0xffff),
            ],
        );

        cpu.step();
        assert_eq!(cpu.registers[8], 0);
        cpu.step();
        assert_eq!(cpu.registers[8], 0);
        cpu.step();
        assert_eq!(cpu.registers[8], 1);
    }

    #[test]
    fn test_andi() {
        let mut cpu = test_cpu(
            &[(7, 0b1100_1101)],
            &[
                // ANDI r8, r7, 0xff
                i_type(0x0c, 8, 7, 0xff),
                // ANDI r8, r7, 0xf0
                i_type(0x0c, 8, 7, 0xf0),
                // ANDI r8, r7, 0xffff
                i_type(0x0c, 8, 7, 0xffff),
            ],
        );

        cpu.step();
        assert_eq!(cpu.registers[8], 0b1100_1101);
        cpu.step();
        assert_eq!(cpu.registers[8], 0b1100_0000);
        cpu.step();
        assert_eq!(cpu.registers[8], 0b1100_1101);
    }

    #[test]
    fn test_ori() {
        let mut cpu = test_cpu(
            &[(7, 0b1100_1101)],
            &[
                // ORI r8, r7, 0xff
                i_type(0x0d, 8, 7, 0xff),
                // ORI r8, r7, 0xf0
                i_type(0x0d, 8, 7, 0xf0),
            ],
        );

        cpu.step();
        assert_eq!(cpu.registers[8], 0b1111_1111);
        cpu.step();
        assert_eq!(cpu.registers[8], 0b1111_1101);
    }

    #[test]
    fn test_xori() {
        let mut cpu = test_cpu(
            &[(7, 0b1100_1101)],
            &[
                // XORI r8, r7, 0xff
                i_type(0x0e, 8, 7, 0xff),
                // XORI r8, r7, 0xf0
                i_type(0x0e, 8, 7, 0xf0),
            ],
        );

        cpu.step();
        assert_eq!(cpu.registers[8], 0b0011_0010);
        cpu.step();
        assert_eq!(cpu.registers[8], 0b0011_1101);
    }
}
