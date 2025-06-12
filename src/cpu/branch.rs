use crate::cpu::{Cpu, Instruction};

impl Cpu {
    /// 00.08 - JR - R-Type
    /// JR rs
    /// PC = GPR[rs]
    pub(super) fn ins_jr(&mut self, instruction: Instruction) {
        let target = self.get_rs(instruction);
        self.branch_target = Some(target);
    }

    /// 00.09 - JALR - R-Type
    /// JALR rs, rd
    /// temp = GPR[rs]
    /// GPR[rd] = PC + 4
    /// PC = temp
    pub(super) fn ins_jalr(&mut self, instruction: Instruction) {
        let target = self.get_rs(instruction);

        self.write_reg(instruction.rd(), self.pc.wrapping_add(4));

        self.branch_target = Some(target);
    }

    /// 01.00 - BLTZ - I-Type
    /// BLTZ rs, offset
    /// if (signed(GPR[rs]) < 0)
    ///     PC = PC + sign_extended(offset) << 2
    pub(super) fn ins_bltz(&mut self, instruction: Instruction) {
        if (self.get_rs(instruction) as i32) < 0 {
            let target =
                self.pc.wrapping_add((instruction.simm16() as u32) << 2);
            self.branch_target = Some(target);
        }
    }

    /// 01.01 - BGEZ - I-Type
    /// BGEZ rs, offset
    /// if (signed(GPR[rs]) >= 0)
    ///     PC = PC + sign_extended(offset) << 2
    pub(super) fn ins_bgez(&mut self, instruction: Instruction) {
        if (self.get_rs(instruction) as i32) >= 0 {
            let target =
                self.pc.wrapping_add((instruction.simm16() as u32) << 2);
            self.branch_target = Some(target);
        }
    }

    /// 01.10 - BLTZAL - I-Type
    /// BLTZAL rs, offset
    /// temp = signed(GPR[rs])
    /// GPR[31] = PC + 4
    /// if (temp < 0)
    ///     PC = PC + sign_extended(offset) << 2
    pub(super) fn ins_bltzal(&mut self, instruction: Instruction) {
        let value = self.get_rs(instruction) as i32;

        self.write_reg(31, self.pc.wrapping_add(4));

        if value < 0 {
            let target =
                self.pc.wrapping_add((instruction.simm16() as u32) << 2);
            self.branch_target = Some(target);
        }
    }

    /// 01.11 - BGEZAL - I-Type
    /// BGEZAL rs, offset
    /// temp = signed(GPR[rs])
    /// GPR[31] = PC + 4
    /// if (temp >= 0)
    ///    PC = PC + sign_extended(offset) << 2
    pub(super) fn ins_bgezal(&mut self, instruction: Instruction) {
        let value = self.get_rs(instruction) as i32;

        self.write_reg(31, self.pc.wrapping_add(4));

        if value >= 0 {
            let target =
                self.pc.wrapping_add((instruction.simm16() as u32) << 2);
            self.branch_target = Some(target);
        }
    }

    /// 02 - J - J-Type
    /// J destination
    /// PC = (PC & 0xf000_0000) | (destination << 2)
    pub(super) fn ins_j(&mut self, instruction: Instruction) {
        let target = (self.pc & 0xf000_0000) | (instruction.jump_target() << 2);
        self.branch_target = Some(target);
    }

    /// 03 - JAL - J-Type
    /// JAL destination
    /// GPR[31] = PC + 4
    /// PC = (PC & 0xf000_0000) | (destination << 2)
    pub(super) fn ins_jal(&mut self, instruction: Instruction) {
        self.write_reg(31, self.pc.wrapping_add(4));

        let target = (self.pc & 0xf000_0000) | (instruction.jump_target() << 2);
        self.branch_target = Some(target);
    }

    /// 04 - BEQ - I-Type
    /// BEQ rs, rt, offset
    /// if (GPR[rs] == GPR[rt])
    ///     PC = PC + sign_extended(offset) << 2
    pub(super) fn ins_beq(&mut self, instruction: Instruction) {
        if self.get_rs(instruction) == self.get_rt(instruction) {
            let target =
                self.pc.wrapping_add((instruction.simm16() as u32) << 2);
            self.branch_target = Some(target);
        }
    }

    /// 05 - BNE - I-Type
    /// BNE rs, rt, offset
    /// if (GPR[rs] != GPR[rt])
    ///    PC = PC + sign_extended(offset) << 2
    pub(super) fn ins_bne(&mut self, instruction: Instruction) {
        if self.get_rs(instruction) != self.get_rt(instruction) {
            let target =
                self.pc.wrapping_add((instruction.simm16() as u32) << 2);
            self.branch_target = Some(target);
        }
    }

    /// 06 - BLEZ - I-Type
    /// BLEZ rs, offset
    /// if (signed(GPR[rs]) <= 0)
    ///     PC = PC + sign_extended(offset) << 2
    pub(super) fn ins_blez(&mut self, instruction: Instruction) {
        if (self.get_rs(instruction) as i32) <= 0 {
            let target =
                self.pc.wrapping_add((instruction.simm16() as u32) << 2);
            self.branch_target = Some(target);
        }
    }

    /// 07 - BGTZ - I-Type
    /// BGTZ rs, offset
    /// if (signed(GPR[rs]) > 0)
    ///     PC = PC + sign_extended(offset) << 2
    pub(super) fn ins_bgtz(&mut self, instruction: Instruction) {
        if (self.get_rs(instruction) as i32) > 0 {
            let target =
                self.pc.wrapping_add((instruction.simm16() as u32) << 2);
            self.branch_target = Some(target);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::cpu::test_utils::*;

    #[test]
    fn test_j() {
        let mut cpu = test_cpu(&[], &[j_type(0x02, 0x3000)]);

        cpu_steps(&mut cpu, 2);

        assert_eq!(cpu.pc, 0x3000);
    }

    #[test]
    fn test_jal() {
        let mut cpu = test_cpu(&[], &[j_type(0x03, 0x3000)]);

        cpu_steps(&mut cpu, 2);

        assert_eq!(cpu.pc, 0x3000);
        assert_eq!(cpu.registers[31], 0x1008); // return address
    }

    #[test]
    fn test_jal_in_delay_slot_of_jal() {
        let mut cpu = test_cpu(
            &[],
            &[
                // JAL 0x2000 (RA = 0x1008)
                j_type(0x03, 0x2000),
                // JAL 0x3000 (RA = 0x100c)
                j_type(0x03, 0x3000),
                // NOP (delay slot)
            ],
        );

        cpu_steps(&mut cpu, 3);

        assert_eq!(cpu.pc, 0x3000);
        assert_eq!(cpu.registers[31], 0x100c); // return address
    }

    #[test]
    fn test_jr() {
        let mut cpu = test_cpu(
            &[(1, 0x0000_2000)],
            &[
                // JR r1
                r_type(0x08, 0, 0, 1),
            ],
        );

        cpu_steps(&mut cpu, 2);

        assert_eq!(cpu.pc, 0x0000_2000);
    }

    #[test]
    fn test_jalr_custom_rd() {
        let mut cpu = test_cpu(
            &[(1, 0x3000)],
            &[
                // JALR r1, r2
                r_type(0x09, 2, 0, 1),
            ],
        );

        cpu_steps(&mut cpu, 2);

        assert_eq!(cpu.pc, 0x3000);
        assert_eq!(cpu.registers[2], 0x1008); // return address
        assert_ne!(cpu.registers[31], 0x1008); // $ra should not be used
    }

    #[test]
    fn test_jalr_rd_equal_rs() {
        // TODO
    }

    #[test]
    fn test_beq_taken() {
        let mut cpu = test_cpu(
            &[(1, 10), (2, 10)],
            &[
                // BEQ r1, r2, offset_taken (4 words = 16 bytes)
                i_type(0x04, 1, 2, 4),
            ],
        );

        cpu_steps(&mut cpu, 2);

        assert_eq!(cpu.pc, 0x1000 + 4 + 16, "BEQ taken PC");
    }

    #[test]
    fn test_beq_taken_negative_offset() {
        let mut cpu = test_cpu(
            &[(1, 10), (2, 10)],
            &[
                // BEQ r1, r2, offset_taken (-32 words = -128 bytes)
                i_type(0x04, 1, 2, 0u16.wrapping_sub(32)),
            ],
        );

        cpu_steps(&mut cpu, 2);

        assert_eq!(cpu.pc, 0x1000 + 4 - 128, "BEQ taken PC");
    }

    #[test]
    fn test_beq_not_taken() {
        let mut cpu = test_cpu(
            &[(1, 10), (2, 20)],
            &[
                // BEQ r1, r2, offset_not_taken (4 words = 16 bytes)
                i_type(0x04, 1, 2, 4),
            ],
        );

        cpu_steps(&mut cpu, 2);

        assert_eq!(cpu.pc, 0x1000 + 8, "BNE not taken PC");
    }

    #[test]
    fn test_bne_taken() {
        let mut cpu = test_cpu(
            &[(1, 10), (2, 20)],
            &[
                // BNE r1, r2, offset_taken (4 words = 16 bytes)
                i_type(0x05, 1, 2, 4),
            ],
        );

        cpu_steps(&mut cpu, 2);

        assert_eq!(cpu.pc, 0x1000 + 4 + 16, "BNE taken PC");
    }

    #[test]
    fn test_bne_not_taken() {
        let mut cpu = test_cpu(
            &[(1, 10), (2, 10)],
            &[
                // BNE r1, r2, offset_not_taken (4 words = 16 bytes)
                i_type(0x05, 1, 2, 4),
            ],
        );

        cpu_steps(&mut cpu, 2);

        assert_eq!(cpu.pc, 0x1000 + 8, "BNE not taken PC");
    }

    #[test]
    fn test_blez_taken_negative() {
        let mut cpu = test_cpu(
            &[(1, -5i32 as u32)],
            &[
                // BLEZ r1, offset_taken (2 words = 8 bytes)
                i_type(0x06, 0, 1, 2),
            ],
        );

        cpu_steps(&mut cpu, 2);

        assert_eq!(cpu.pc, 0x1000 + 4 + 8, "BLEZ taken (negative) PC");
    }

    #[test]
    fn test_blez_taken_zero() {
        let mut cpu = test_cpu(
            &[],
            &[
                // BLEZ r1, offset_taken (2 words = 8 bytes)
                i_type(0x06, 0, 1, 2),
            ],
        );

        cpu_steps(&mut cpu, 2);

        assert_eq!(cpu.pc, 0x1000 + 4 + 8, "BLEZ taken (zero) PC");
    }

    #[test]
    fn test_blez_not_taken_positive() {
        let mut cpu = test_cpu(
            &[(1, 5)],
            &[
                // BLEZ r1, offset_not_taken (2 words = 8 bytes)
                i_type(0x06, 0, 1, 2),
            ],
        );

        cpu_steps(&mut cpu, 2);

        assert_eq!(cpu.pc, 0x1000 + 8, "BLEZ not taken (positive) PC");
    }

    #[test]
    fn test_bgtz_taken_positive() {
        let mut cpu = test_cpu(
            &[(1, 5)],
            &[
                // BGTZ r1, offset_taken (3 words = 12 bytes)
                i_type(0x07, 0, 1, 3),
            ],
        );

        cpu_steps(&mut cpu, 2);

        assert_eq!(cpu.pc, 0x1000 + 4 + (3 << 2), "BGTZ taken (positive) PC");
    }

    #[test]
    fn test_bgtz_not_taken_zero() {
        let mut cpu = test_cpu(
            &[],
            &[
                // BGTZ r1, offset_not_taken (3 words = 12 bytes)
                i_type(0x07, 0, 1, 3),
            ],
        );

        cpu_steps(&mut cpu, 2);

        assert_eq!(cpu.pc, 0x1000 + 8, "BGTZ not taken (zero) PC");
    }

    #[test]
    fn test_bgtz_not_taken_negative() {
        let mut cpu = test_cpu(
            &[(1, -5i32 as u32)],
            &[
                // BGTZ r1, offset_not_taken (3 words = 12 bytes)
                i_type(0x07, 0, 1, 3),
            ],
        );

        cpu_steps(&mut cpu, 2);

        assert_eq!(cpu.pc, 0x1000 + 8, "BGTZ not taken (negative) PC");
    }

    #[test]
    fn test_bltz_taken() {
        let mut cpu = test_cpu(
            &[(1, -1i32 as u32)],
            &[
                // BLTZ (rt = 0) r1, offset
                i_type(0x01, 0, 1, 5),
            ],
        );

        cpu_steps(&mut cpu, 2);

        assert_eq!(cpu.pc, 0x1000 + 4 + (5 << 2), "BLTZ taken PC");
    }

    #[test]
    fn test_bltz_not_taken() {
        let mut cpu = test_cpu(
            &[],
            &[
                // BLTZ (rt = 0) r1, offset
                i_type(0x01, 0, 1, 5),
            ],
        );

        cpu_steps(&mut cpu, 2);

        assert_eq!(cpu.pc, 0x1000 + 8, "BLTZ not taken PC");
    }

    #[test]
    fn test_bgez_taken() {
        let mut cpu = test_cpu(
            &[],
            &[
                // BGEZ (rt = 1) r1, offset
                i_type(0x01, 1, 1, 6),
            ],
        );

        cpu_steps(&mut cpu, 2);

        assert_eq!(cpu.pc, 0x1000 + 4 + 24, "BGEZ taken PC");
    }

    #[test]
    fn test_bgez_not_taken() {
        let mut cpu = test_cpu(
            &[
                (1, -1i32 as u32), // not >= 0
            ],
            &[
                // BGEZ (rt = 1) r1, offset
                i_type(0x01, 1, 1, 6),
            ],
        );

        cpu_steps(&mut cpu, 2);

        assert_eq!(cpu.pc, 0x1000 + 8, "BGEZ not taken PC");
    }

    #[test]
    fn test_bltzal_taken() {
        let mut cpu = test_cpu(
            &[(1, -1i32 as u32)],
            &[
                // BLTZAL (rt = 0x10) r1, offset
                i_type(0x01, 0x10, 1, 7),
            ],
        );

        cpu_steps(&mut cpu, 2);

        assert_eq!(cpu.pc, 0x1000 + 4 + 28, "BLTZAL taken PC");
        assert_eq!(cpu.registers[31], 0x1000 + 8, "BLTZAL RA");
    }

    #[test]
    fn test_bltzal_not_taken() {
        let mut cpu = test_cpu(
            &[],
            &[
                // BLTZAL (rt = 0x10) r1, offset
                i_type(0x01, 0x10, 1, 7),
            ],
        );

        cpu_steps(&mut cpu, 2);

        assert_eq!(cpu.pc, 0x1000 + 8, "BLTZAL not taken PC");

        // RA is set regardless of the branch taken or not.
        assert_ne!(
            cpu.registers[31], 0xdeadbeef,
            "BLTZAL RA not taken, $ra unchanged"
        );
    }

    #[test]
    fn test_bltzal_rs_31() {
        // TODO
    }

    #[test]
    fn test_bgezal_taken() {
        let mut cpu = test_cpu(
            &[],
            &[
                // BGEZAL (rt = 0x11) r1, offset
                i_type(0x01, 0x11, 1, 8),
            ],
        );

        cpu_steps(&mut cpu, 2);

        assert_eq!(cpu.pc, 0x1000 + 4 + 32, "BGEZAL taken PC");
        assert_eq!(cpu.registers[31], 0x1000 + 8, "BGEZAL RA");
    }

    #[test]
    fn test_bgezal_not_taken() {
        let mut cpu = test_cpu(
            &[
                (1, -1i32 as u32), // not >= 0
            ],
            &[
                // BGEZAL (rt = 0x11) r1, offset
                i_type(0x01, 0x11, 1, 8),
            ],
        );

        cpu_steps(&mut cpu, 2);

        assert_eq!(cpu.pc, 0x1000 + 8, "BGEZAL not taken PC");
        assert_ne!(
            cpu.registers[31], 0xdeadbeef,
            "BGEZAL RA not taken, $ra unchanged"
        );
    }

    #[test]
    fn test_bgezal_rs_31() {
        // TODO
    }

    #[test]
    fn test_weird_encodings() {
        // rt & 0x1e == 0x10 are the AL versions of the branch instructions.
        // any other value, even if 0x1n, is the non-AL version.
    }
}
