//[ new-file
use super::{AccessSize, Cpu, Instruction};

impl Cpu {
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
    //[ ins-sw
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
    //] ins-sw
}
//] new-file
//[ !omit
#[cfg(test)]
mod tests {
    use crate::{AccessSize, cpu::test_utils::*};

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
