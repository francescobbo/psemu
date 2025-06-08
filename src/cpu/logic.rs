//[ new-logic
use crate::cpu::{Cpu, Instruction};

impl Cpu {
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
