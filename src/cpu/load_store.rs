use super::{Cpu, Instruction};

impl Cpu {
    /// 20 - LB - I-type
    /// LB rt, offset(rs)
    /// GPR[rt] = sign_extend(Memory[rs + offset, 8-bit])
    pub(super) fn ins_lb(&mut self, instr: Instruction) {
        let address = self.target_address(instr);
        let value = self.ram.read8(address) as i8 as u32;

        self.write_reg(instr.rt(), value);
    }

    /// 21 - LH - I-type
    /// LH rt, offset(rs)
    /// GPR[rt] = sign_extend(Memory[rs + offset, 16-bit])
    pub(super) fn ins_lh(&mut self, instr: Instruction) {
        let address = self.target_address(instr);
        let value = self.ram.read16(address) as i16 as u32;

        self.write_reg(instr.rt(), value);
    }

    /// 23 - LW - I-type
    /// LW rt, offset(rs)
    /// GPR[rt] = Memory[rs + offset, 32-bit]
    pub(super) fn ins_lw(&mut self, instr: Instruction) {
        let address = self.target_address(instr);
        let value = self.ram.read32(address);

        self.write_reg(instr.rt(), value);
    }

    /// 24 - LBU - I-type
    /// LBU rt, offset(rs)
    /// GPR[rt] = zero_extend(Memory[rs + offset, 8-bit])
    pub(super) fn ins_lbu(&mut self, instr: Instruction) {
        let address = self.target_address(instr);
        let value = self.ram.read8(address) as u32;

        self.write_reg(instr.rt(), value);
    }

    /// 25 - LHU - I-type
    /// LHU rt, offset(rs)
    /// GPR[rt] = zero_extend(Memory[rs + offset, 16-bit])
    pub(super) fn ins_lhu(&mut self, instr: Instruction) {
        let address = self.target_address(instr);
        let value = self.ram.read16(address) as u32;

        self.write_reg(instr.rt(), value);
    }

    /// 28 - SB - I-type
    /// SB rt, offset(rs)
    /// Memory[rs + sign_extended(offset), 8-bit] = GPR[rt]
    pub(super) fn ins_sb(&mut self, instr: Instruction) {
        let address = self.target_address(instr);
        let value = self.get_rt(instr) as u8;

        self.ram.write8(address, value);
    }

    /// 29 - SH - I-type
    /// SH rt, offset(rs)
    /// Memory[rs + sign_extended(offset), 16-bit] = GPR[rt]
    pub(super) fn ins_sh(&mut self, instr: Instruction) {
        let address = self.target_address(instr);
        let value = self.get_rt(instr) as u16;

        self.ram.write16(address, value);
    }

    /// 2B - SW - I-type
    /// SW rt, offset(rs)
    /// Memory[rs + sign_extended(offset), 32-bit] = GPR[rt]
    pub(super) fn ins_sw(&mut self, instr: Instruction) {
        let address = self.target_address(instr);
        let value = self.get_rt(instr);

        self.ram.write32(address, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sw() {
        let mut cpu = Cpu::new();
        cpu.registers[7] = 0x1000;
        cpu.registers[8] = 0x12345678;

        cpu.execute(Instruction(0xace8_0000)); // SW r8, 0(r7)

        assert_eq!(cpu.ram.read32(0x1000), 0x12345678);

        // Test that the store was done in little-endian order
        assert_eq!(cpu.ram.read16(0x1000), 0x5678);

        // Test a store to a negative offset
        cpu.execute(Instruction(0xace8_fffc)); // SW r8, -4(r7)
        assert_eq!(cpu.ram.read32(0x0ffc), 0x12345678);
    }

    #[test]
    fn test_sh() {
        let mut cpu = Cpu::new();
        cpu.registers[7] = 0x1000;
        cpu.registers[8] = 0x12345678;

        cpu.execute(Instruction(0xa4e8_0000)); // SH r8, 0(r7)

        assert_eq!(cpu.ram.read16(0x1000), 0x5678);

        // Test that the store was done in little-endian order
        assert_eq!(cpu.ram.read8(0x1000), 0x78);

        // Test a store to a negative offset
        cpu.execute(Instruction(0xa4e8_fffe)); // SH r8, -2(r7)
        assert_eq!(cpu.ram.read8(0x0fff), 0x56);
    }

    #[test]
    fn test_sb() {
        let mut cpu = Cpu::new();
        cpu.registers[7] = 0x1000;
        cpu.registers[8] = 0x12345678;

        cpu.execute(Instruction(0xa0e8_0000)); // SB r8, 0(r7)

        assert_eq!(cpu.ram.read8(0x1000), 0x78);

        // Test a store to a negative offset
        cpu.execute(Instruction(0xa0e8_ffff)); // SB r8, -1(r7)
        assert_eq!(cpu.ram.read8(0x0fff), 0x78);
    }
}
