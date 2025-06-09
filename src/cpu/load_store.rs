//[ new-file
use super::{AccessSize, Cpu, Instruction};

impl Cpu {
    //[ ins-lb
    /// 20 - LB - I-type
    /// LB rt, offset(rs)
    /// GPR[rt] = sign_extend(Memory[rs + sign_extend(offset), 8-bit])
    pub(super) fn ins_lb(&mut self, instr: Instruction) {
        let address = self.target_address(instr);

        match self.read_memory(address, AccessSize::Byte) {
            Ok(value) => {
                // Sign-extend the byte value
                let value = value as i8 as u32;
                self.write_reg(instr.rt(), value);
            }
            Err(_) => self.exception("Memory read error"),
        }
    }
    //] ins-lb
    //[ load-store-impl
    //[ ins-lh-lw-lbu-lhu-stub
    /// 21 - LH - I-type
    /// LH rt, offset(rs)
    /// GPR[rt] = sign_extend(Memory[rs + sign_extend(offset), 16-bit])
    pub(super) fn ins_lh(&mut self, instr: Instruction) {
        // Your implementation here
    }

    /// 23 - LW - I-type
    /// LW rt, offset(rs)
    /// GPR[rt] = Memory[rs + sign_extend(offset)]
    pub(super) fn ins_lw(&mut self, instr: Instruction) {
        let address = self.target_address(instr);

        match self.read_memory(address, AccessSize::Word) {
            Ok(value) => self.write_reg(instr.rt(), value),
            Err(_) => self.exception("Memory read error"),
        }
    }

    /// 24 - LBU - I-type
    /// LBU rt, offset(rs)
    /// GPR[rt] = Memory[rs + sign_extend(offset), 8-bit]
    pub(super) fn ins_lbu(&mut self, instr: Instruction) {
        // Your implementation here
    }

    /// 25 - LHU - I-type
    /// LHU rt, offset(rs)
    /// GPR[rt] = Memory[rs + sign_extend(offset), 16-bit]
    pub(super) fn ins_lhu(&mut self, instr: Instruction) {
        // Your implementation here
    }
    //] ins-lh-lw-lbu-lhu-stub

    //[ sb-sh-stub
    /// 28 - SB - I-type
    /// SB rt, offset(rs)
    /// Memory[rs + sign_extend(offset), 8-bit] = GPR[rt] & 0xff
    pub(super) fn ins_sb(&mut self, instr: Instruction) {
        let address = self.target_address(instr);
        let value = self.get_rt(instr);
        if self.write_memory(address, value, AccessSize::Byte).is_err() {
            self.exception("Memory write error");
        }
    }

    /// 29 - SH - I-type
    /// SH rt, offset(rs)
    /// Memory[rs + sign_extend(offset), 16-bit] = GPR[rt] & 0xffff
    pub(super) fn ins_sh(&mut self, instr: Instruction) {
        let address = self.target_address(instr);
        let value = self.get_rt(instr);

        if self
            .write_memory(address, value, AccessSize::HalfWord)
            .is_err()
        {
            self.exception("Memory write error");
        }
    }
    //] sb-sh-stub
    //] load-store-impl
    /// 2B - SW - I-type
    /// SW rt, offset(rs)
    /// Memory[rs + sign_extend(offset)] = GPR[rt]
    pub(super) fn ins_sw(&mut self, instr: Instruction) {
        let address = self.target_address(instr);

        if let Err(_) =
            self.write_memory(address, self.get_rt(instr), AccessSize::Word)
        {
            self.exception("Memory write error");
        }
    }
}
//] new-file
//[ !omit
#[cfg(test)]
mod tests {
    use crate::{AccessSize, cpu::test_utils::*};

    #[test]
    fn test_lb() {
        let mut cpu = test_cpu(
            &[(7, 0x4000)],
            &[
                i_type(0x20, 8, 7, 0xffff), // Load from address 0x3fff
                i_type(0x20, 9, 7, 1),      // Load from address 0x4001
            ],
        );

        // Set up memory with a byte at the target address
        cpu.write_memory(0x3fff, 0x78, AccessSize::Byte).unwrap();
        cpu.write_memory(0x4001, 0x88, AccessSize::Byte).unwrap();

        cpu_steps(&mut cpu, 2);

        assert_eq!(cpu.registers[8], 0x78);
        assert_eq!(cpu.registers[9], 0xffff_ff88); // Sign-extended value
    }

    #[test]
    fn test_lh() {
        let mut cpu = test_cpu(
            &[(7, 0x4000)],
            &[
                i_type(0x21, 8, 7, 0xfffe), // Load from address 0x3ffe
                i_type(0x21, 9, 7, 2),      // Load from address 0x4002
                i_type(0x21, 10, 7, 4), // Load half-word from address 0x4004
                i_type(0x21, 11, 7, 6), // Load word from address 0x4006
            ],
        );

        // Set up memory with an half-word at the target address
        cpu.write_memory(0x3ffe, 0x78, AccessSize::Byte).unwrap();
        cpu.write_memory(0x4002, 0x88, AccessSize::Byte).unwrap();
        cpu.write_memory(0x4004, 0x8001, AccessSize::HalfWord)
            .unwrap();
        cpu.write_memory(0x4006, 0x6001, AccessSize::HalfWord)
            .unwrap();

        cpu_steps(&mut cpu, 4);

        assert_eq!(cpu.registers[8], 0x78);
        assert_eq!(cpu.registers[9], 0x88); // Byte does not get sign-extended
        assert_eq!(cpu.registers[10], 0xffff_8001); // Half-word is sign-extended
        assert_eq!(cpu.registers[11], 0x6001); // Sign-extended but positive
    }

    #[test]
    fn test_lw() {
        let mut cpu = test_cpu(
            &[(7, 0x4000)],
            &[
                i_type(0x23, 8, 7, 0xfffc), // Load from address 0x3ffc
                i_type(0x23, 9, 7, 4),      // Load from address 0x4008
            ],
        );

        cpu.write_memory(0x3ffc, 0xcafecafe, AccessSize::Word)
            .unwrap();
        cpu.write_memory(0x4004, 0x12345678, AccessSize::Word)
            .unwrap();

        cpu_steps(&mut cpu, 2);

        assert_eq!(cpu.registers[8], 0xcafecafe);
        assert_eq!(cpu.registers[9], 0x12345678);
    }

    #[test]
    fn test_lbu() {
        let mut cpu = test_cpu(
            &[(7, 0x4000)],
            &[
                i_type(0x24, 8, 7, 0xffff), // Load from address 0x3fff
                i_type(0x24, 9, 7, 1),      // Load from address 0x4001
            ],
        );

        // Set up memory with a byte at the target address
        cpu.write_memory(0x3fff, 0x78, AccessSize::Byte).unwrap();
        cpu.write_memory(0x4001, 0x88, AccessSize::Byte).unwrap();

        cpu_steps(&mut cpu, 2);

        assert_eq!(cpu.registers[8], 0x78);
        assert_eq!(cpu.registers[9], 0x88); // No sign-extension for LBU
    }

    #[test]
    fn test_lhu() {
        let mut cpu = test_cpu(
            &[(7, 0x4000)],
            &[
                i_type(0x25, 8, 7, 0xfffe), // Load from address 0x3ffe
                i_type(0x25, 9, 7, 2),      // Load from address 0x4002
            ],
        );

        // Set up memory with a half-word at the target address
        cpu.write_memory(0x3ffe, 0x7800, AccessSize::HalfWord)
            .unwrap();
        cpu.write_memory(0x4002, 0x8800, AccessSize::HalfWord)
            .unwrap();

        cpu_steps(&mut cpu, 2);

        assert_eq!(cpu.registers[8], 0x7800);
        assert_eq!(cpu.registers[9], 0x8800); // No sign-extension for LHU
    }

    #[test]
    fn test_sb() {
        let mut cpu = test_cpu(
            &[(7, 0x4000), (8, 0x12345678)],
            &[i_type(0x28, 8, 7, 0xffff)], // Store byte at address 0x3fff
        );

        cpu.step();

        // Check if the byte was stored correctly
        assert_eq!(
            cpu.read_memory(0x3fff, AccessSize::Byte).unwrap(),
            0x78 // Last byte of 0x12345678
        );
    }

    #[test]
    fn test_sh() {
        let mut cpu = test_cpu(
            &[(7, 0x4000), (8, 0x12345678)],
            &[i_type(0x29, 8, 7, 0xfffe)], // Store half-word at address 0x3ffe
        );

        cpu.step();

        // Check if the half-word was stored correctly
        assert_eq!(
            cpu.read_memory(0x3ffe, AccessSize::HalfWord).unwrap(),
            0x5678 // Last two bytes of 0x12345678
        );
    }

    #[test]
    fn test_sw() {
        let mut cpu = test_cpu(
            &[(7, 0x4000), (8, 0x12345678)],
            &[i_type(0x2b, 8, 7, 0xfffc)], // Store word at address 0x3ffc
        );

        cpu.step();

        // Check if the word was stored correctly
        assert_eq!(
            cpu.read_memory(0x3ffc, AccessSize::Word).unwrap(),
            0x12345678
        );
    }
}
//] !omit
