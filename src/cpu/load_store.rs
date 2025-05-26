use super::{Cpu, DelayedLoad, Instruction};
use crate::bus::AccessSize;

impl Cpu {
    fn delayed_load(&mut self, target: usize, value: u32) {
        if target == 0 {
            // If the target is $0, we don't need to do anything.
            return;
        }

        self.load_delay = Some(DelayedLoad { target, value });
    }

    /// 20 - LB - I-type
    /// LB rt, offset(rs)
    /// GPR[rt] = sign_extend(Memory[rs + offset, 8-bit])
    pub(super) fn ins_lb(&mut self, instr: Instruction) {
        let address = self.target_address(instr);

        match self.read_memory(address, AccessSize::Byte) {
            Ok(value) => {
                // Sign-extend the byte value
                let value = value as i8 as u32;
                self.delayed_load(instr.rt(), value);
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
                self.delayed_load(instr.rt(), value);
            }
            Err(_) => self.exception("Memory read error")
        }
    }

    /// 22 - LWL - I-type
    /// LWL rt, offset(rs)
    /// Loads the left (most significant) bytes of a word from an unaligned
    /// memory address.
    pub(super) fn ins_lwl(&mut self, instr: Instruction) {
        let addr = self.target_address(instr);

        // Perform an aligned load of 4 bytes
        let aligned_word = match self.read_memory(addr & !3, AccessSize::Word) {
            Ok(value) => value,
            Err(_) => {
                self.exception("Memory read error");
                return;
            }
        };

        // Get the current value of the register (even if it's delayed)
        let reg = self.get_possibly_delayed_reg(instr.rt());

        // Depending on the address offset, we need to shift the loaded word
        let value = match addr & 3 {
            0 => (reg & 0x00ffffff) | (aligned_word << 24),
            1 => (reg & 0x0000ffff) | (aligned_word << 16),
            2 => (reg & 0x000000ff) | (aligned_word << 8),
            3 => aligned_word,
            _ => unreachable!(),
        };

        self.delayed_load(instr.rt(), value);
    }

    /// 23 - LW - I-type
    /// LW rt, offset(rs)
    /// GPR[rt] = Memory[rs + offset, 32-bit]
    pub(super) fn ins_lw(&mut self, instr: Instruction) {
        let address = self.target_address(instr);

        match self.read_memory(address, AccessSize::Word) {
            Ok(value) => self.delayed_load(instr.rt(), value),
            Err(_) => self.exception("Memory read error")
        }
    }

    /// 24 - LBU - I-type
    /// LBU rt, offset(rs)
    /// GPR[rt] = zero_extend(Memory[rs + offset, 8-bit])
    pub(super) fn ins_lbu(&mut self, instr: Instruction) {
        let address = self.target_address(instr);

        match self.read_memory(address, AccessSize::Byte) {
            Ok(value) => self.delayed_load(instr.rt(), value),
            Err(_) => self.exception("Memory read error")
        }
    }

    /// 25 - LHU - I-type
    /// LHU rt, offset(rs)
    /// GPR[rt] = zero_extend(Memory[rs + offset, 16-bit])
    pub(super) fn ins_lhu(&mut self, instr: Instruction) {
        let address = self.target_address(instr);

        match self.read_memory(address, AccessSize::HalfWord) {
            Ok(value) => self.delayed_load(instr.rt(), value),
            Err(_) => self.exception("Memory read error")
        }
    }

    /// 26 - LWR - I-type
    /// LWR rt, offset(rs)
    /// Loads the right (least significant) bytes of a word from an unaligned
    /// memory address.
    pub(super) fn ins_lwr(&mut self, instr: Instruction) {
        let addr = self.target_address(instr);

        // Perform an aligned load of 4 bytes
        let aligned_word = match self.read_memory(addr & !3, AccessSize::Word) {
            Ok(value) => value,
            Err(_) => {
                self.exception("Memory read error");
                return;
            }
        };

        let reg = self.get_possibly_delayed_reg(instr.rt());

        let value = match addr & 3 {
            0 => aligned_word,
            1 => (reg & 0xff000000) | (aligned_word >> 8),
            2 => (reg & 0xffff0000) | (aligned_word >> 16),
            3 => (reg & 0xffffff00) | (aligned_word >> 24),
            _ => unreachable!(),
        };

        self.delayed_load(instr.rt(), value);
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

    /// 2A - SWL - I-type
    /// SWL rt, offset(rs)
    /// Stores the left (most significant) bytes of a word to an unaligned
    /// memory address.
    pub(super) fn ins_swl(&mut self, instr: Instruction) {
        let addr = self.target_address(instr);

        // Perform an aligned read of 4 bytes Note that the real SWL does not
        // read the memory, the merging is performed by the RAM chip. We shall
        // not raise an exception if the read fails.
        let aligned_word = self
            .read_memory(addr & !3, AccessSize::Word)
            .unwrap_or_default();

        // Get the current value of the register
        let reg = self.get_rt(instr);

        // Depending on the address offset, we need to shift the loaded word
        let value = match addr & 3 {
            0 => (aligned_word & 0xffffff00) | (reg >> 24),
            1 => (aligned_word & 0xffff0000) | (reg >> 16),
            2 => (aligned_word & 0xff000000) | (reg >> 8),
            3 => aligned_word,
            _ => unreachable!(),
        };

        // Write the modified value back to memory, aligned to a word boundary
        if self.write_memory(addr & !3, value, AccessSize::Word).is_err() {
            self.exception("Memory write error");
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

    /// 2E - SWR - I-type
    /// SWR rt, offset(rs)
    /// Stores the right (least significant) bytes of a word to an unaligned
    /// memory address.
    pub(super) fn ins_swr(&mut self, instr: Instruction) {
        let addr = self.target_address(instr);

        // Perform an aligned read of 4 bytes
        let aligned_word = self
            .read_memory(addr & !3, AccessSize::Word)
            .unwrap_or_default();

        // Get the current value of the register
        let reg = self.get_rt(instr);

        // Depending on the address offset, we need to shift the loaded word
        let value = match addr & 3 {
            0 => reg,
            1 => (aligned_word & 0x000000ff) | (reg << 8),
            2 => (aligned_word & 0x0000ffff) | (reg << 16),
            3 => (aligned_word & 0x00ffffff) | (reg << 24),
            _ => unreachable!(),
        };

        // Write the modified value back to memory, aligned to a word boundary
        if self.write_memory(addr & !3, value, AccessSize::Word).is_err() {
            self.exception("Memory write error");
        }
    }

    fn get_possibly_delayed_reg(&self, index: usize) -> u32 {
        if let Some(DelayedLoad { target, value }) = self.current_load_delay {
            if target == index {
                return value;
            }
        }

        self.registers[index]
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

        assert_eq!(
            cpu.read_memory(0x2000, AccessSize::HalfWord).unwrap(),
            0x5678
        );
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
        assert_eq!(
            cpu.read_memory(0x1000, AccessSize::Word).unwrap(),
            0x1234_5678
        );
        assert_eq!(
            cpu.read_memory(0x0ffc, AccessSize::Word).unwrap(),
            0x1234_5678
        );

        // Test that the store was done in little-endian order
        assert_eq!(
            cpu.read_memory(0x1000, AccessSize::HalfWord).unwrap(),
            0x5678
        );
    }

    /// Instruction in delay slot reads OLD value of LW's destination register.
    /// Instruction after delay slot reads NEW (loaded) value.
    #[test]
    fn lw_has_delay_slot() {
        let mem_addr = 0x0a00;
        let value_in_memory = 0xcafe_beef;
        let initial_t0_val = 0x11111111;

        let mut cpu = test_cpu(
            &[
                (8, initial_t0_val), // $t0
                (16, mem_addr),      // $s0
            ],
            &[
                // LW $t0, 0($s0)
                i_type(0x23, 8, 16, 0),
                // ADDU $t1, $t0, $0
                r_type(0x21, 9, 0, 8),
                // ADDU $t2, $t0, $0
                r_type(0x21, 10, 0, 8),
            ],
        );

        cpu.write_memory(mem_addr, value_in_memory, AccessSize::Word)
            .unwrap();

        // Execute LW $t0, 0($s0)
        // - $t0 should still be initial_t0_val at the *end* of this step's GPR state.
        // - pending_load should be Some((8, value_in_memory)).
        cpu.step();
        assert_eq!(cpu.registers[8], initial_t0_val);
        assert!(cpu.load_delay.is_some());
        if let Some(load) = &cpu.load_delay {
            assert_eq!(load.target, 8);
            assert_eq!(load.value, value_in_memory);
        }

        // Execute ADDU $t1, $t0, $0 (Delay Slot Instruction)
        // - pending_load from LW is committed at the END of the cycle for instruction D (0x10C).
        // - So, during execution of instruction D (0x10C), $t0 is still initial_t0_val.
        cpu.step(); // This step executes the delay slot instruction ADDU $t1, $t0, $0

        // r9 = old $t0 + $0 ==> old $t0
        assert_eq!(cpu.registers[9], initial_t0_val);

        // r8 ($t0) has now been updated to value_in_memory
        assert_eq!(cpu.registers[8], value_in_memory);
        assert!(cpu.load_delay.is_none());

        // Execute ADDU $t2, $t0, $0 (Instruction L+2, after Delay Slot)
        cpu.step();
        assert_eq!(cpu.registers[10], value_in_memory);
        assert_eq!(cpu.registers[8], value_in_memory);
    }

    /// LW's destination register is $0. No pending load should be set, $0
    /// should remain 0.
    #[test]
    fn lw_to_r0_nop() {
        let mem_addr = 0x0a00;
        let value_in_memory = 0xcafe_beef;

        let mut cpu = test_cpu(
            &[
                (16, mem_addr), // $s0
            ],
            &[
                // LW $0, 0($s0)
                i_type(0x23, 0, 16, 0),
                // NOP
                0,
                // ADDU $t1, $0, $0
                r_type(0x21, 9, 0, 0),
            ],
        );
        cpu.write_memory(mem_addr as u32, value_in_memory, AccessSize::Word)
            .unwrap();

        cpu.step();
        assert_eq!(cpu.registers[0], 0);
        assert!(cpu.load_delay.is_none());

        cpu.step();
        assert_eq!(cpu.registers[0], 0);

        cpu.step();
        assert_eq!(cpu.registers[9], 0);
        assert_eq!(cpu.registers[0], 0);
    }

    /// Delay slot is discarded if a following instruction overwrites the
    /// destination register.
    #[test]
    fn lw_delay_slot_overwritten() {
        let mem_addr = 0x0a00;
        let value_in_memory = 0xabcd_ef01;
        let initial_t0_val = 0x12345678;

        let mut cpu = test_cpu(
            &[
                (8, initial_t0_val), // $t0
                (16, mem_addr),      // $s0
            ],
            &[
                // LW $t0, 0($s0)
                i_type(0x23, 8, 16, 0),
                // ADDIU $t0, $0, 0x5a5a
                i_type(0x09, 8, 0, 0x5a5a),
                // ADDU $t2, $t0, $0
                r_type(0x21, 10, 0, 8),
            ],
        );
        cpu.write_memory(mem_addr, value_in_memory, AccessSize::Word)
            .unwrap();

        cpu.step(); // Execute LW $t0, ...
        // After this step, $t0 still initial_t0_val, pending_load is Some for $t0
        assert_eq!(cpu.registers[8], initial_t0_val);

        cpu.step(); // Execute ADDIU $t0 (Delay Slot)
        assert_eq!(cpu.registers[8], 0x5a5a);

        cpu.step(); // Execute ADDU $t2, $t0, $0 (Instruction L+2)
        assert_eq!(cpu.registers[8], 0x5a5a);
        assert_eq!(cpu.registers[10], 0x5a5a);
    }

    /// Two consecutive LW instructions. Both should load correctly, each into
    /// its own destination register, and with its own delay.
    #[test]
    fn lw_two_consecutive() {
        let mem_addr = 0x0a00;
        let value_in_memory1 = 0xabcd_ef01;
        let value_in_memory2 = 0x1234_5678;

        let mut cpu = test_cpu(
            &[
                (8, 0xcafe_cafe), // $t0
                (9, 0xcafe_cafe), // $t1
                (16, mem_addr),   // $s0
            ],
            &[
                // LW $t0, 0($s0)
                i_type(0x23, 8, 16, 0),
                // LW $t1, 4($s0)
                i_type(0x23, 9, 16, 4),
                // ADDU $t2, $t0, $t1
                r_type(0x21, 10, 9, 8),
            ],
        );
        cpu.write_memory(mem_addr, value_in_memory1, AccessSize::Word)
            .unwrap();
        cpu.write_memory(mem_addr + 4, value_in_memory2, AccessSize::Word)
            .unwrap();

        cpu.step(); // Execute LW $t0, ...
        assert_eq!(cpu.registers[8], 0xcafe_cafe);

        cpu.step(); // Execute LW $t1
        assert_eq!(cpu.registers[8], value_in_memory1);
        assert_eq!(cpu.registers[9], 0xcafe_cafe);

        cpu.step(); // Execute ADDU $t2
        assert_eq!(cpu.registers[8], value_in_memory1);
        assert_eq!(cpu.registers[9], value_in_memory2);
        assert_eq!(
            cpu.registers[10],
            value_in_memory1.wrapping_add(0xcafe_cafe)
        );
    }
}
