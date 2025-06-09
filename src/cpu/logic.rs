//[ new-logic
use crate::cpu::{Cpu, Instruction};

impl Cpu {
    //[ ins-r-type-shifts
    /// 00.00 - SLL - R-Type
    /// SLL rd, rt, shamt
    /// GPR[rd] = GPR[rt] << shamt
    pub(super) fn ins_sll(&mut self, instruction: Instruction) {
        // Your code here
    }

    /// 00.02 - SRL - R-Type
    /// SRL rd, rt, shamt
    /// GPR[rd] = GPR[rt] >> shamt
    pub(super) fn ins_srl(&mut self, instruction: Instruction) {
        // Your code here
    }

    /// 00.03 - SRA - R-Type
    /// SRA rd, rt, shamt
    /// GPR[rd] = signed(GPR[rt]) >> shamt
    pub(super) fn ins_sra(&mut self, instruction: Instruction) {
        // Your code here
    }

    /// 00.04 - SLLV - R-Type
    /// SLLV rd, rt, rs
    /// GPR[rd] = GPR[rt] << (GPR[rs] & 0x1f)
    pub(super) fn ins_sllv(&mut self, instruction: Instruction) {
        // Your code here
    }

    /// 00.06 - SRLV - R-Type
    /// SRLV rd, rt, rs
    /// GPR[rd] = GPR[rt] >> (GPR[rs] & 0x1f)
    pub(super) fn ins_srlv(&mut self, instruction: Instruction) {
        // Your code here
    }

    /// 00.07 - SRAV - R-Type
    /// SRAV rd, rt, rs
    /// GPR[rd] = signed(GPR[rt]) >> (GPR[rs] & 0x1f)
    pub(super) fn ins_srav(&mut self, instruction: Instruction) {
        // Your code here
    }
    //] ins-r-type-shifts
    //[ ins-r-type-logic
    /// 00.24 - AND - R-Type
    /// AND rd, rs, rt
    /// GPR[rd] = GPR[rs] & GPR[rt]
    pub(super) fn ins_and(&mut self, instruction: Instruction) {
        // Your code here
    }

    /// 00.25 - OR - R-Type
    /// OR rd, rs, rt
    /// GPR[rd] = GPR[rs] | GPR[rt]
    pub(super) fn ins_or(&mut self, instruction: Instruction) {
        // Your code here
    }

    /// 00.26 - XOR - R-Type
    /// XOR rd, rs, rt
    /// GPR[rd] = GPR[rs] ^ GPR[rt]
    pub(super) fn ins_xor(&mut self, instruction: Instruction) {
        // Your code here
    }

    /// 00.27 - NOR - R-Type
    /// NOR rd, rs, rt
    /// GPR[rd] = ~(GPR[rs] | GPR[rt])
    pub(super) fn ins_nor(&mut self, instruction: Instruction) {
        // Your code here
    }

    /// 00.2A - SLT - R-Type
    /// SLT rd, rs, rt
    /// GPR[rd] = (signed(GPR[rs]) < signed(GPR[rt])) ? 1 : 0
    pub(super) fn ins_slt(&mut self, instruction: Instruction) {
        // Your code here
    }

    /// 00.2B - SLTU - R-Type
    /// SLTU rd, rs, rt
    /// GPR[rd] = (GPR[rs] < GPR[rt]) ? 1 : 0
    pub(super) fn ins_sltu(&mut self, instruction: Instruction) {
        // Your code here
    }
    //] ins-r-type-logic

    //[ logic-instructions
    //[ slti-stubs
    /// 0A - SLTI - I-type
    /// SLTI rt, rs, immediate
    /// GPR[rt] = (signed(GPR[rs]) < sign_extend(immediate)) ? 1 : 0
    pub fn ins_slti(&mut self, instruction: Instruction) {
        let value = instruction.simm16();
        let result = ((self.get_rs(instruction) as i32) < value) as u32;

        self.write_reg(instruction.rt(), result)
    }

    /// 0B - SLTIU - I-type
    /// SLTIU rt, rs, immediate
    /// GPR[rt] = (GPR[rs] < unsigned(sign_extend(immediate))) ? 1 : 0
    pub fn ins_sltiu(&mut self, instruction: Instruction) {
        let value = instruction.simm16() as u32;
        let result = (self.get_rs(instruction) < value) as u32;

        self.write_reg(instruction.rt(), result)
    }
    //] slti-stubs
    //[ logic-instructions
    /// 0C - ANDI - I-Type
    /// ANDI rt, rs, immediate
    /// GPR[rt] = GPR[rs] & immediate
    pub(super) fn ins_andi(&mut self, instr: Instruction) {
        self.write_reg(instr.rt(), self.get_rs(instr) & instr.imm16());
    }

    /// 0D - ORI - I-Type
    /// ORI rt, rs, immediate
    /// GPR[rt] = GPR[rs] | immediate
    pub(super) fn ins_ori(&mut self, instr: Instruction) {
        self.write_reg(instr.rt(), self.get_rs(instr) | instr.imm16());
    }

    /// 0E - XORI - I-Type
    /// XORI rt, rs, immediate
    /// GPR[rt] = GPR[rs] ^ immediate
    pub(super) fn ins_xori(&mut self, instr: Instruction) {
        self.write_reg(instr.rt(), self.get_rs(instr) ^ instr.imm16());
    }
    //] logic-instructions
    //[ ins-lui
    /// 0F - LUI - I-Type
    /// LUI rt, immediate
    /// GPR[rt] = immediate << 16
    pub(super) fn ins_lui(&mut self, instr: Instruction) {
        self.write_reg(instr.rt(), instr.imm16() << 16);
    }
    //] ins-lui
}
//] new-logic
//[ !omit
#[cfg(test)]
mod tests {
    use crate::cpu::test_utils::*;

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
