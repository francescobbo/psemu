use crate::ram::AccessSize;

use super::{Cpu, Instruction};

impl Cpu {
    /// 20 - LB - I-type
    /// LB rt, offset(rs)
    /// GPR[rt] = sign_extend(Memory[rs + offset, 8-bit])
    pub(super) fn ins_lb(&mut self, instr: Instruction) {
        let address = self.target_address(instr);
        
        match self.read_memory(address, AccessSize::Byte) {
            Ok(value) => {
                // Sign-extend the byte value
                let value = value as i8 as u32;
                self.write_reg(instr.rt(), value);
            }
            Err(_) => self.exception("Memory read error")
        }
    }

    /// 21 - LH - I-type
    /// LH rt, offset(rs)
    /// GPR[rt] = sign_extend(Memory[rs + offset, 16-bit])
    pub(super) fn ins_lh(&mut self, instr: Instruction) {
        let address = self.target_address(instr);
        
        match self.read_memory(address, AccessSize::HalfWord) {
            Ok(value) => {
                // Sign-extend the half-word value
                let value = value as i16 as u32;
                self.write_reg(instr.rt(), value);
            }
            Err(_) => self.exception("Memory read error")
        }
    }

    /// 23 - LW - I-type
    /// LW rt, offset(rs)
    /// GPR[rt] = Memory[rs + offset, 32-bit]
    pub(super) fn ins_lw(&mut self, instr: Instruction) {
        let address = self.target_address(instr);
        
        match self.read_memory(address, AccessSize::Word) {
            Ok(value) => self.write_reg(instr.rt(), value),
            Err(_) => self.exception("Memory read error")
        }
    }

    /// 24 - LBU - I-type
    /// LBU rt, offset(rs)
    /// GPR[rt] = zero_extend(Memory[rs + offset, 8-bit])
    pub(super) fn ins_lbu(&mut self, instr: Instruction) {
        let address = self.target_address(instr);
        
        match self.read_memory(address, AccessSize::Byte) {
            Ok(value) => self.write_reg(instr.rt(), value),
            Err(_) => self.exception("Memory read error")
        }
    }

    /// 25 - LHU - I-type
    /// LHU rt, offset(rs)
    /// GPR[rt] = zero_extend(Memory[rs + offset, 16-bit])
    pub(super) fn ins_lhu(&mut self, instr: Instruction) {
        let address = self.target_address(instr);

        match self.read_memory(address, AccessSize::HalfWord) {
            Ok(value) => self.write_reg(instr.rt(), value),
            Err(_) => self.exception("Memory read error")
        }
    }

    /// 28 - SB - I-type
    /// SB rt, offset(rs)
    /// Memory[rs + sign_extended(offset), 8-bit] = GPR[rt]
    pub(super) fn ins_sb(&mut self, instr: Instruction) {
        let address = self.target_address(instr);
        let value = self.get_rt(instr);

        if self.write_memory(address, value, AccessSize::Byte).is_err() {
            self.exception("Memory write error")
        }
    }

    /// 29 - SH - I-type
    /// SH rt, offset(rs)
    /// Memory[rs + sign_extended(offset), 16-bit] = GPR[rt]
    pub(super) fn ins_sh(&mut self, instr: Instruction) {
        let address = self.target_address(instr);
        let value = self.get_rt(instr);

        if self.write_memory(address, value, AccessSize::HalfWord).is_err() {
            self.exception("Memory write error")
        }
    }

    /// 2B - SW - I-type
    /// SW rt, offset(rs)
    /// Memory[rs + sign_extended(offset), 32-bit] = GPR[rt]
    pub(super) fn ins_sw(&mut self, instr: Instruction) {
        let address = self.target_address(instr);
        let value = self.get_rt(instr);

        if self.write_memory(address, value, AccessSize::Word).is_err() {
            self.exception("Memory write error")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::test_utils::*;

    #[test]
    fn test_sb() {
        let mut cpu = test_cpu(
            &[(7, 0x2000), (8, 0x12345678)],
            &[
                // SB r8, 0(r7)
                i_type(0x28, 8, 7, 0),
                // SB r8, -1(r7)
                i_type(0x28, 8, 7, 0xffff),
            ],
        );

        cpu_steps(&mut cpu, 2);

        assert_eq!(cpu.read_memory(0x2000, AccessSize::Byte).unwrap(), 0x78);
        assert_eq!(cpu.read_memory(0x1fff, AccessSize::Byte).unwrap(), 0x78);
    }

    #[test]
    fn test_sh() {
        let mut cpu = test_cpu(
            &[(7, 0x2000), (8, 0x12345678)],
            &[
                // SH r8, 0(r7)
                i_type(0x29, 8, 7, 0),
                // SH r8, -2(r7)
                i_type(0x29, 8, 7, 0xfffe),
            ],
        );

        cpu_steps(&mut cpu, 2);

        assert_eq!(cpu.read_memory(0x2000, AccessSize::HalfWord).unwrap(), 0x5678);
        assert_eq!(cpu.read_memory(0x1fff, AccessSize::Byte).unwrap(), 0x56);

        // Test that the store was done in little-endian order
        assert_eq!(cpu.read_memory(0x2000, AccessSize::Byte).unwrap(), 0x78);
    }

    #[test]
    fn test_sw() {
        let mut cpu = test_cpu(
            &[(7, 0x1000), (8, 0x1234_5678)],
            &[
                // SW r8, 0(r7)
                i_type(0x2b, 8, 7, 0),
                // SW r8, -4(r7)
                i_type(0x2b, 8, 7, 0xfffc),
            ],
        );

        cpu_steps(&mut cpu, 2);
        assert_eq!(cpu.read_memory(0x1000, AccessSize::Word).unwrap(), 0x1234_5678);
        assert_eq!(cpu.read_memory(0x0ffc, AccessSize::Word).unwrap(), 0x1234_5678);

        // Test that the store was done in little-endian order
        assert_eq!(cpu.read_memory(0x1000, AccessSize::HalfWord).unwrap(), 0x5678);
    }
}
