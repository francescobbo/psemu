use crate::cpu::{Cpu, Instruction};

impl Cpu {
    /// 00.08 - JR - R-Type
    /// JR rs
    /// PC = GPR[rs]
    pub(super) fn ins_jr(&mut self, instruction: Instruction) {
        let target = self.get_rs(instruction);
        self.next_pc = Some(target);
    }

    /// 00.09 - JALR - R-Type
    /// JALR rs
    /// GPR[rd] = PC + 4
    /// PC = GPR[rs]
    pub(super) fn ins_jalr(&mut self, instruction: Instruction) {
        self.write_reg(instruction.rd(), self.pc.wrapping_add(4));

        let target = self.get_rs(instruction);
        self.next_pc = Some(target);
    }

    /// 01.00 - BLTZ - I-Type
    /// BLTZ rs, offset
    /// if (signed(GPR[rs]) < 0)
    ///     PC = PC + sign_extended(offset << 2)
    pub(super) fn ins_bltz(&mut self, instruction: Instruction) {
        if (self.get_rs(instruction) as i32) < 0 {
            let target = self.pc.wrapping_add((instruction.simm16() as u32) << 2);
            self.next_pc = Some(target);
        }
    }

    /// 01.01 - BGEZ - I-Type
    /// BGEZ rs, offset
    /// if (signed(GPR[rs]) >= 0)
    ///     PC = PC + sign_extended(offset << 2)
    pub(super) fn ins_bgez(&mut self, instruction: Instruction) {
        if (self.get_rs(instruction) as i32) >= 0 {
            let target = self.pc.wrapping_add((instruction.simm16() as u32) << 2);
            self.next_pc = Some(target);
        }
    }

    /// 01.10 - BLTZAL - I-Type
    /// BLTZAL rs, offset
    /// GPR[31] = PC + 4
    /// if (signed(GPR[rs]) < 0)
    ///     PC = PC + sign_extended(offset << 2)
    pub(super) fn ins_bltzal(&mut self, instruction: Instruction) {
        self.write_reg(31, self.pc.wrapping_add(4));

        if (self.get_rs(instruction) as i32) < 0 {
            let target = self.pc.wrapping_add((instruction.simm16() as u32) << 2);
            self.next_pc = Some(target);
        }
    }

    /// 01.11 - BGEZAL - I-Type
    /// BGEZAL rs, offset
    /// GPR[31] = PC + 4
    /// if (signed(GPR[rs]) >= 0)
    ///    PC = PC + sign_extended(offset << 2)
    pub(super) fn ins_bgezal(&mut self, instruction: Instruction) {
        self.write_reg(31, self.pc.wrapping_add(4));

        if (self.get_rs(instruction) as i32) >= 0 {
            let target = self.pc.wrapping_add((instruction.simm16() as u32) << 2);
            self.next_pc = Some(target);
        }
    }

    /// 02 - J - J-Type
    /// J destination
    /// PC = (PC & 0xf000_0000) | (destination << 2)
    pub(super) fn ins_j(&mut self, instruction: Instruction) {
        let target = (self.pc & 0xf000_0000) | (instruction.jump_target() << 2);
        self.next_pc = Some(target);
    }

    /// 03 - JAL - J-Type
    /// JAL destination
    /// GPR[31] = PC + 4
    /// PC = (PC & 0xf000_0000) | (destination << 2)
    pub(super) fn ins_jal(&mut self, instruction: Instruction) {
        self.write_reg(31, self.pc.wrapping_add(4));

        let target = (self.pc & 0xf000_0000) | (instruction.jump_target() << 2);
        self.next_pc = Some(target);
    }

    /// 04 - BEQ - I-Type
    /// BEQ rs, rt, offset
    /// if (GPR[rs] == GPR[rt])
    ///     PC = PC + sign_extended(offset << 2)
    pub(super) fn ins_beq(&mut self, instruction: Instruction) {
        if self.get_rs(instruction) == self.get_rt(instruction) {
            let target = self.pc.wrapping_add((instruction.simm16() as u32) << 2);
            self.next_pc = Some(target);
        }
    }

    /// 05 - BNE - I-Type
    /// BNE rs, rt, offset
    /// if (GPR[rs] != GPR[rt])
    ///    PC = PC + sign_extended(offset << 2)
    pub(super) fn ins_bne(&mut self, instruction: Instruction) {
        if self.get_rs(instruction) != self.get_rt(instruction) {
            let target = self.pc.wrapping_add((instruction.simm16() as u32) << 2);
            self.next_pc = Some(target);
        }
    }

    /// 06 - BLEZ - I-Type
    /// BLEZ rs, offset
    /// if (signed(GPR[rs]) <= 0)
    ///     PC = PC + sign_extended(offset << 2)
    pub(super) fn ins_blez(&mut self, instruction: Instruction) {
        if (self.get_rs(instruction) as i32) <= 0 {
            let target = self.pc.wrapping_add((instruction.simm16() as u32) << 2);
            self.next_pc = Some(target);
        }
    }

    /// 07 - BGTZ - I-Type
    /// BGTZ rs, offset
    /// if (signed(GPR[rs]) > 0)
    ///     PC = PC + sign_extended(offset << 2)
    pub(super) fn ins_bgtz(&mut self, instruction: Instruction) {
        if (self.get_rs(instruction) as i32) > 0 {
            let target = self.pc.wrapping_add((instruction.simm16() as u32) << 2);
            self.next_pc = Some(target);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn nop() -> u32 {
        0x00000000
    }

    // Helper to create a CPU, load instructions, and run 2 steps
    fn build(initial_pc: u32, instructions: &[u32]) -> Cpu {
        let mut cpu = Cpu::new();

        for (i, &instr_word) in instructions.iter().enumerate() {
            cpu.ram.write32(initial_pc + (i * 4) as u32, instr_word);
        }

        cpu.pc = initial_pc;
        cpu
    }

    // Helper to create a J-Type instruction word
    fn j_type(opcode: u32, target_pseudo_addr: u32) -> u32 {
        (opcode << 26) | (target_pseudo_addr >> 2) // target_pseudo_addr is word address
    }

    // Helper to create an I-Type instruction word
    fn i_type(opcode: u32, rs: usize, rt: usize, immediate: i16) -> u32 {
        (opcode << 26) | ((rs as u32) << 21) | ((rt as u32) << 16) | (immediate as u16 as u32)
    }

    // Helper to create an R-Type instruction word
    fn r_type(funct: u32, rs: usize, rt: usize, rd: usize, shamt: usize) -> u32 {
        // opcode for SPECIAL is 0x00
        ((rs as u32) << 21)
            | ((rt as u32) << 16)
            | ((rd as u32) << 11)
            | ((shamt as u32) << 6)
            | funct
    }

    // --- J ---
    // Opcode: 0x02
    #[test]
    fn test_j() {
        // J to 0x0000_1000. PC = 0x100. Delay slot PC = 0x104.
        // Target calculation: (0x104 & 0xF0000000) | (0x1000 >> 2 << 2) = 0x00001000
        let j_instr = j_type(0x02, 0x0000_1000);
        let mut cpu = build(0x100, &[j_instr, nop()]);

        cpu.step();
        cpu.step();

        assert_eq!(cpu.pc, 0x0000_1000, "J PC should be target");
    }

    // --- JAL ---
    // Opcode: 0x03
    #[test]
    fn test_jal() {
        // JAL to 0x0000_1000. PC = 0x100. Delay slot PC = 0x104. RA = 0x108.
        // Target calculation: (0x104 & 0xF0000000) | (0x1000 >> 2 << 2) = 0x00001000
        let jal_instr = j_type(0x03, 0x0000_1000);
        let mut cpu = build(0x100, &[jal_instr, nop()]);

        cpu.step();
        cpu.step();

        assert_eq!(cpu.pc, 0x0000_1000, "JAL PC should be target");
        assert_eq!(cpu.registers[31], 0x100 + 8, "JAL RA should be PC+8");
    }

    #[test]
    fn jal_in_delay_slot_of_jal() {
        // PC=0x100: JAL 0x2000 (Target1 = 0x2000, RA1 = 0x108)
        // PC=0x104: JAL 0x3000 (Target2 = 0x3000, RA2 = 0x10C (PC of JAL in delay slot + 8))
        // PC=0x108: NOP      (Delay slot for the second JAL)
        // Expected final PC = 0x3000
        // Expected $ra = 0x10C (from the second JAL)

        let initial_pc = 0x0100;
        let jal1_instr = j_type(0x03, 0x2000);
        let jal2_instr = j_type(0x03, 0x3000);

        let mut cpu = build(initial_pc, &[jal1_instr, jal2_instr, nop()]);
        cpu.registers[31] = 0xdeadbeef; // sentinel to check overwritten anyway

        cpu.step(); // Exec JAL1. Sets next_pc=Some(0x2000). $ra=0x108. cpu.pc=0x104.
        cpu.step(); // Exec JAL2 (delay slot of JAL1).
        cpu.step(); // Exec NOP (delay slot of JAL2). cpu.pc becomes 0x3000.

        assert_eq!(cpu.pc, 0x3000, "JAL in JAL's delay slot: PC check");
        assert_eq!(
            cpu.registers[31],
            0x0104 + 8,
            "JAL in JAL's delay slot: $ra check"
        );
    }

    // --- JR ---
    // Funct: 0x08 (Opcode 0x00)
    #[test]
    fn test_jr() {
        // JR to address in R1 (0x0000_2000). PC = 0x100.
        let jr_instr = r_type(0x08, 1, 0, 0, 0); // JR $1

        let mut cpu = build(0x100, &[jr_instr, nop()]);
        cpu.registers[1] = 0x0000_2000;

        cpu.step();
        cpu.step();

        assert_eq!(cpu.pc, 0x0000_2000, "JR PC should be R1's value");
    }

    // --- JALR ---
    // Funct: 0x09 (Opcode 0x00)
    #[test]
    fn test_jalr_default_rd() {
        // JALR $1. rd defaults to $31. Target in R1 (0x3000). PC = 0x100. RA = 0x108.
        let jalr_instr = r_type(0x09, 1, 0, 31, 0); // JALR $31, $1 (assembler might do JALR $1)

        let mut cpu = build(0x100, &[jalr_instr, nop()]);
        cpu.registers[1] = 0x0000_3000;

        cpu.step();
        cpu.step();

        assert_eq!(cpu.pc, 0x0000_3000, "JALR PC");
        assert_eq!(cpu.registers[31], 0x100 + 8, "JALR RA ($31)");
    }

    #[test]
    fn test_jalr_custom_rd() {
        // JALR $2, $1. Target in R1 (0x3000). PC = 0x100. Store RA in R2.
        let jalr_instr = r_type(0x09, 1, 0, 2, 0); // JALR $2, $1

        let mut cpu = build(0x100, &[jalr_instr, nop()]);
        cpu.registers[1] = 0x0000_3000;

        cpu.step();
        cpu.step();

        assert_eq!(cpu.pc, 0x0000_3000, "JALR custom rd PC");
        assert_eq!(cpu.registers[2], 0x100 + 8, "JALR RA in R2");
        assert_ne!(
            cpu.registers[31],
            0x100 + 8,
            "JALR RA not in R31 unless R2 was R31"
        );
    }

    // --- BEQ ---
    // Opcode: 0x04
    #[test]
    fn test_beq_taken() {
        // BEQ $1, $2, offset_taken (4 words = 16 bytes)
        // $1 = 10, $2 = 20. They are not equal. Branch taken.
        // PC = 0x100. Delay slot at 0x104. Target = 0x104 + 16 = 0x114
        let beq_instr = i_type(0x04, 1, 2, 4); // Offset is 4 words

        let mut cpu = build(0x100, &[beq_instr, nop()]);
        cpu.registers[1] = 10;
        cpu.registers[2] = 10;

        cpu.step(); // Execute BEQ
        cpu.step(); // Execute delay slot NOP, then PC jumps

        assert_eq!(cpu.pc, 0x100 + 4 + 16, "BEQ taken PC");
    }

    #[test]
    fn test_beq_taken_negative_offset() {
        // BEQ $1, $2, offset_taken (4 words = 16 bytes)
        // $1 = 10, $2 = 20. They are not equal. Branch taken.
        // PC = 0x100. Delay slot at 0x104. Target = 0x104 + 16 = 0x84
        let beq_instr = i_type(0x04, 1, 2, -32);

        let mut cpu = build(0x100, &[beq_instr, nop()]);
        cpu.registers[1] = 10;
        cpu.registers[2] = 10;

        cpu.step(); // Execute BEQ
        cpu.step(); // Execute delay slot NOP, then PC jumps

        assert_eq!(cpu.pc, 0x100 + 4 - 32 * 4, "BEQ taken PC");
    }

    #[test]
    fn test_beq_not_taken() {
        // BEQ $1, $2, offset_not_taken
        // $1 = 10, $2 = 10. They are equal. Branch NOT taken.
        // PC = 0x100. Delay slot at 0x104. Next instr at 0x108.
        let beq_instr = i_type(0x04, 1, 2, 4);

        let mut cpu = build(0x100, &[beq_instr, nop(), nop()]);
        cpu.registers[1] = 10;
        cpu.registers[2] = 20;

        cpu.step(); // Execute BEQ
        cpu.step(); // Execute delay slot NOP

        assert_eq!(cpu.pc, 0x100 + 8, "BNE not taken PC");
    }

    // --- BNE ---
    // Opcode: 0x05
    #[test]
    fn test_bne_taken() {
        // BNE $1, $2, offset_taken (4 words = 16 bytes)
        // $1 = 10, $2 = 20. They are not equal. Branch taken.
        // PC = 0x100. Delay slot at 0x104. Target = 0x104 + 16 = 0x114
        let bne_instr = i_type(0x05, 1, 2, 4); // Offset is 4 words

        let mut cpu = build(0x100, &[bne_instr, nop()]);
        cpu.registers[1] = 10;
        cpu.registers[2] = 20;

        cpu.step(); // Execute BNE
        cpu.step(); // Execute delay slot NOP, then PC jumps

        assert_eq!(cpu.pc, 0x100 + 4 + 16, "BNE taken PC");
    }

    #[test]
    fn test_bne_not_taken() {
        // BNE $1, $2, offset_not_taken
        // $1 = 10, $2 = 10. They are equal. Branch NOT taken.
        // PC = 0x100. Delay slot at 0x104. Next instr at 0x108.
        let bne_instr = i_type(0x05, 1, 2, 4);

        let mut cpu = build(0x100, &[bne_instr, nop(), nop()]);
        cpu.registers[1] = 10;
        cpu.registers[2] = 10;

        cpu.step(); // Execute BNE
        cpu.step(); // Execute delay slot NOP

        assert_eq!(cpu.pc, 0x100 + 8, "BNE not taken PC");
    }

    // --- BLEZ ---
    // Opcode: 0x06 (rt field is 0)
    #[test]
    fn test_blez_taken_negative() {
        // BLEZ $1, offset. $1 = -5 (FFFFFFFBs). Branch taken.
        // PC=0x100. Target = 0x104 + (2<<2) = 0x10C
        let blez_instr = i_type(0x06, 1, 0, 2); // rt must be 0

        let mut cpu = build(0x100, &[blez_instr, nop()]);
        cpu.registers[1] = -5i32 as u32;

        cpu.step();
        cpu.step();

        assert_eq!(cpu.pc, 0x100 + 4 + 8, "BLEZ taken (negative) PC");
    }

    #[test]
    fn test_blez_taken_zero() {
        // BLEZ $1, offset. $1 = 0. Branch taken.
        let blez_instr = i_type(0x06, 1, 0, 2);

        let mut cpu = build(0x100, &[blez_instr, nop()]);
        cpu.registers[1] = 0;

        cpu.step();
        cpu.step();

        assert_eq!(cpu.pc, 0x100 + 4 + 8, "BLEZ taken (zero) PC");
    }

    #[test]
    fn test_blez_not_taken_positive() {
        // BLEZ $1, offset. $1 = 5. Branch NOT taken.
        let blez_instr = i_type(0x06, 1, 0, 2);

        let mut cpu = build(0x100, &[blez_instr, nop(), nop()]);
        cpu.registers[1] = 5;

        cpu.step();
        cpu.step();

        assert_eq!(cpu.pc, 0x100 + 8, "BLEZ not taken (positive) PC");
    }

    // --- BGTZ ---
    // Opcode: 0x07 (rt field is 0)
    #[test]
    fn test_bgtz_taken_positive() {
        // BGTZ $1, offset. $1 = 5. Branch taken.
        let bgtz_instr = i_type(0x07, 1, 0, 3);

        let mut cpu = build(0x100, &[bgtz_instr, nop()]);
        cpu.registers[1] = 5;

        cpu.step();
        cpu.step();

        assert_eq!(cpu.pc, 0x100 + 4 + (3 << 2), "BGTZ taken (positive) PC");
    }

    #[test]
    fn test_bgtz_not_taken_zero() {
        let bgtz_instr = i_type(0x07, 1, 0, 3);

        let mut cpu = build(0x100, &[bgtz_instr, nop(), nop()]);
        cpu.registers[1] = 0;

        cpu.step();
        cpu.step();

        assert_eq!(cpu.pc, 0x100 + 8, "BGTZ not taken (zero) PC");
    }

    #[test]
    fn test_bgtz_not_taken_negative() {
        let bgtz_instr = i_type(0x07, 1, 0, 3);

        let mut cpu = build(0x100, &[bgtz_instr, nop(), nop()]);
        cpu.registers[1] = -5i32 as u32;

        cpu.step();
        cpu.step();

        assert_eq!(cpu.pc, 0x100 + 8, "BGTZ not taken (negative) PC");
    }

    // BLTZ: rt = 0x00
    #[test]
    fn test_bltz_taken() {
        // BLTZ $1, offset. $1 = -1. rt field is 0x00.
        let bltz_instr = i_type(0x01, 1, 0x00, 5);

        let mut cpu = build(0x100, &[bltz_instr, nop()]);
        cpu.registers[1] = -1i32 as u32;

        cpu.step();
        cpu.step();

        assert_eq!(cpu.pc, 0x100 + 4 + (5 << 2), "BLTZ taken PC");
    }

    #[test]
    fn test_bltz_not_taken() {
        let bltz_instr = i_type(0x01, 1, 0x00, 5);

        let mut cpu = build(0x100, &[bltz_instr, nop(), nop()]);
        cpu.registers[1] = 0; // Not less than zero

        cpu.step();
        cpu.step();

        assert_eq!(cpu.pc, 0x100 + 8, "BLTZ not taken PC");
    }

    // BGEZ: rt = 0x01
    #[test]
    fn test_bgez_taken() {
        // BGEZ $1, offset. $1 = 0. rt field is 0x01.
        let bgez_instr = i_type(0x01, 1, 0x01, 6);

        let mut cpu = build(0x100, &[bgez_instr, nop()]);
        cpu.registers[1] = 0;

        cpu.step();
        cpu.step();

        assert_eq!(cpu.pc, 0x100 + 4 + 24, "BGEZ taken PC");
    }

    #[test]
    fn test_bgez_not_taken() {
        let bgez_instr = i_type(0x01, 1, 0x01, 6);

        let mut cpu = build(0x100, &[bgez_instr, nop()]);
        cpu.registers[1] = -1i32 as u32; // Not >= zero

        cpu.step();
        cpu.step();

        assert_eq!(cpu.pc, 0x100 + 8, "BGEZ not taken PC");
    }

    // BLTZAL: rt = 0x10
    #[test]
    fn test_bltzal_taken() {
        // BLTZAL $1, offset. $1 = -1. rt field is 0x10.
        let bltzal_instr = i_type(0x01, 1, 0x10, 7);

        let mut cpu = build(0x100, &[bltzal_instr, nop()]);
        cpu.registers[1] = -1i32 as u32;

        cpu.step();
        cpu.step();

        assert_eq!(cpu.pc, 0x100 + 4 + 28, "BLTZAL taken PC");
        assert_eq!(cpu.registers[31], 0x100 + 8, "BLTZAL RA");
    }

    #[test]
    fn test_bltzal_not_taken() {
        // BLTZAL $1, offset. $1 = 0.
        let bltzal_instr = i_type(0x01, 1, 0x10, 7);

        let mut cpu = build(0x100, &[bltzal_instr, nop()]);
        cpu.registers[1] = 0;
        cpu.registers[31] = 0xdeadbeef; // sentinel to check overwritten anyway

        cpu.step();
        cpu.step();

        assert_eq!(cpu.pc, 0x100 + 8, "BLTZAL not taken PC");
        assert_ne!(
            cpu.registers[31], 0xdeadbeef,
            "BLTZAL RA not taken, $ra unchanged"
        );
    }

    // BGEZAL: rt = 0x11
    #[test]
    fn test_bgezal_taken() {
        // BGEZAL $1, offset. $1 = 0. rt field is 0x11.
        let bgezal_instr = i_type(0x01, 1, 0x11, 8);

        let mut cpu = build(0x100, &[bgezal_instr, nop()]);
        cpu.registers[1] = 0;

        cpu.step();
        cpu.step();

        assert_eq!(cpu.pc, 0x100 + 4 + 32, "BGEZAL taken PC");
        assert_eq!(cpu.registers[31], 0x100 + 8, "BGEZAL RA");
    }

    #[test]
    fn test_bgezal_not_taken() {
        // BGEZAL $1, offset. $1 = -1.
        let bgezal_instr = i_type(0x01, 1, 0x11, 8);

        let mut cpu = build(0x100, &[bgezal_instr, nop()]);
        cpu.registers[1] = -1i32 as u32;
        cpu.registers[31] = 0xdeadbeef; // sentinel

        cpu.step();
        cpu.step();

        assert_eq!(cpu.pc, 0x100 + 8, "BGEZAL not taken PC");
        assert_ne!(
            cpu.registers[31], 0xdeadbeef,
            "BGEZAL RA not taken, $ra unchanged"
        );
    }
}
