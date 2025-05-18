use super::{Cpu, Instruction};

impl Cpu {
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
        println!("ORI: rs = {:#X}, rt = {:#X}, immediate = {:#X}", self.get_rs(instr), instr.rt(), instr.imm16());
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
}
