use super::{Cpu, Instruction};

impl Cpu {
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
