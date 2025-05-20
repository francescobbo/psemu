use super::{Cpu, DelayedLoad, Instruction};

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
        let value = self.ram.read8(address) as i8 as u32;

        self.delayed_load(instr.rt(), value);
    }

    /// 21 - LH - I-type
    /// LH rt, offset(rs)
    /// GPR[rt] = sign_extend(Memory[rs + offset, 16-bit])
    pub(super) fn ins_lh(&mut self, instr: Instruction) {
        let address = self.target_address(instr);
        let value = self.ram.read16(address) as i16 as u32;

        self.delayed_load(instr.rt(), value);
    }

    /// 23 - LW - I-type
    /// LW rt, offset(rs)
    /// GPR[rt] = Memory[rs + offset, 32-bit]
    pub(super) fn ins_lw(&mut self, instr: Instruction) {
        let address = self.target_address(instr);
        let value = self.ram.read32(address);

        self.delayed_load(instr.rt(), value);
    }

    /// 24 - LBU - I-type
    /// LBU rt, offset(rs)
    /// GPR[rt] = zero_extend(Memory[rs + offset, 8-bit])
    pub(super) fn ins_lbu(&mut self, instr: Instruction) {
        let address = self.target_address(instr);
        let value = self.ram.read8(address) as u32;

        self.delayed_load(instr.rt(), value);
    }

    /// 25 - LHU - I-type
    /// LHU rt, offset(rs)
    /// GPR[rt] = zero_extend(Memory[rs + offset, 16-bit])
    pub(super) fn ins_lhu(&mut self, instr: Instruction) {
        let address = self.target_address(instr);
        let value = self.ram.read16(address) as u32;

        self.delayed_load(instr.rt(), value);
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

    use crate::cpu::{Cpu, Instruction}; // Adjust if your paths are different

    const NOP: u32 = 0x00000000; // SLL $0, $0, 0

    // Helper to create an I-Type instruction word
    // LW opcode = 0x23 (35)
    // ADDIU opcode = 0x09 (9)
    fn i_type_instr(opcode: u32, rs: usize, rt: usize, immediate: i16) -> u32 {
        (opcode << 26) | ((rs as u32) << 21) | ((rt as u32) << 16) | (immediate as u16 as u32)
    }

    // Helper to create an R-Type instruction word
    // ADDU funct = 0x21 (33)
    fn r_type_instr(funct: u32, rs: usize, rt: usize, rd: usize, shamt: usize) -> u32 {
        // Opcode for SPECIAL R-Type is 0x00
        ((rs as u32) << 21)
            | ((rt as u32) << 16)
            | ((rd as u32) << 11)
            | ((shamt as u32) << 6)
            | funct
    }

    fn build(initial_pc: u32, instructions: &[u32]) -> Cpu {
        let mut cpu = Cpu::new(); // Assumes Cpu::new() initializes pc, pending_load, etc.
        cpu.pc = initial_pc;
        for (i, &instr_word) in instructions.iter().enumerate() {
            cpu.ram.write32(initial_pc + (i * 4) as u32, instr_word);
        }

        cpu
    }

    // Test 1: Instruction in delay slot reads OLD value of LW's destination register.
    // Instruction after delay slot reads NEW (loaded) value.
    #[test]
    fn lw_delay_slot_uses_old_value_next_uses_new() {
        let mem_addr_for_lw = 0x0A00;
        let value_in_memory = 0xCAFEBABE;
        let initial_t0_val = 0x11111111;

        // Registers:
        // $t0 (8)  = initial_t0_val, then destination of LW
        // $s0 (16) = mem_addr_for_lw
        // $t1 (9)  = result of delay slot operation (should use old $t0)
        // $t2 (10) = result of operation after delay slot (should use new $t0)

        let instructions = [
            // 0x108: LW $t0, 0($s0)  --- This is Instruction L ---
            i_type_instr(0x23, 16, 8, 0), // LW $t0 (rt=8), 0($s0 (rs=16))
            // 0x10C: ADDU $t1, $t0, $0 --- This is Instruction D (Delay Slot) ---
            r_type_instr(0x21, 8, 0, 9, 0), // ADDU $t1 (rd=9), $t0 (rs=8), $zero (rt=0)
            // 0x110: ADDU $t2, $t0, $0 --- This is Instruction L+2 (After Delay Slot) ---
            r_type_instr(0x21, 8, 0, 10, 0), // ADDU $t2 (rd=10), $t0 (rs=8), $zero (rt=0)
        ];

        let mut cpu = build(0x100, &instructions);
        cpu.registers[8] = initial_t0_val;
        cpu.registers[16] = mem_addr_for_lw;

        cpu.ram.write32(mem_addr_for_lw, value_in_memory);
        assert!(
            cpu.load_delay.is_none(),
            "Pending load should be None before LW executes"
        );

        // Execute LW $t0, 0($s0)
        // - $t0 should still be initial_t0_val at the *end* of this step's GPR state.
        // - pending_load should be Some((8, value_in_memory)).
        cpu.step();
        assert_eq!(
            cpu.registers[8], initial_t0_val,
            "LW executed: $t0 should still be old value immediately after LW's cycle"
        );
        assert!(
            cpu.load_delay.is_some(),
            "LW executed: pending_load should be Some"
        );
        if let Some(load) = &cpu.load_delay {
            assert_eq!(
                load.target, 8,
                "LW executed: pending_load has correct rd_idx"
            );
            assert_eq!(
                load.value, value_in_memory,
                "LW executed: pending_load has correct value"
            );
        }

        // Execute ADDU $t1, $t0, $0 (Delay Slot Instruction)
        // - pending_load from LW is committed at the END of the cycle for instruction D (0x10C).
        // - So, during execution of instruction D (0x10C), $t0 is still initial_t0_val.
        cpu.step(); // This step executes the delay slot instruction ADDU $t1, $t0, $0

        // r9 = old $t0 + $0 ==> old $t0
        assert_eq!(
            cpu.registers[9], initial_t0_val,
            "Delay slot ADDU: $t1 should use OLD $t0 value"
        );

        // r8 ($t0) has now been updated to value_in_memory
        assert_eq!(
            cpu.registers[8], value_in_memory,
            "Delay slot cycle: $t0 should be updated to loaded value"
        );
        assert!(
            cpu.load_delay.is_none(),
            "Delay slot cycle: pending_load should be None after delay slot instruction"
        );

        // Execute ADDU $t2, $t0, $0 (Instruction L+2, after Delay Slot)
        cpu.step();
        assert_eq!(
            cpu.registers[10], value_in_memory,
            "After delay slot ADDU: $t2 should use NEW $t0 value"
        );
        assert_eq!(
            cpu.registers[8], value_in_memory,
            "After delay slot cycle: $t0 itself should be NEW value"
        );
    }

    // Test 2: LW's destination register is $0.
    // No pending load should be set, $0 should remain 0.
    #[test]
    fn lw_to_r0_is_nop_and_no_pending_load() {
        let mem_addr_for_lw = 0x0A00;
        let value_in_memory = 0xCAFEBABE;

        // $s0 (16) = mem_addr_for_lw
        // $t1 (9) = used after delay slot to check $0
        let instructions = [
            i_type_instr(0x09, 0, 16, mem_addr_for_lw as i16), // ADDIU $s0, $0, mem_addr
            i_type_instr(0x23, 16, 0, 0),                      // LW $0 (rt=0), 0($s0 (rs=16))
            NOP,                                               // Delay slot
            r_type_instr(0x21, 0, 0, 9, 0), // ADDU $t1, $0, $0 (check if $0 is still 0)
        ];

        let mut cpu = build(0x100, &instructions);
        cpu.ram.write32(mem_addr_for_lw, value_in_memory);

        cpu.step(); // ADDIU $s0

        cpu.step(); // Execute LW $0, ...
        assert_eq!(
            cpu.registers[0], 0,
            "LW to $0: $0 must remain 0 after LW cycle"
        );
        assert!(
            cpu.load_delay.is_none(),
            "LW to $0: pending_load should be None"
        );

        cpu.step();
        assert_eq!(
            cpu.registers[0], 0,
            "Delay slot for LW $0: $0 must remain 0"
        );
        // pending_load was None, so nothing committed from it.

        cpu.step();
        assert_eq!(
            cpu.registers[9], 0,
            "After LW $0 and delay: $t1 should be 0 (from $0 + $0)"
        );
        assert_eq!(cpu.registers[0], 0, "$0 must always be 0");
    }

    // Test 3: Delay slot instruction does NOT use LW's destination register.
    // Load should still complete correctly for instructions after the delay slot.
    #[test]
    fn lw_delay_slot_unrelated_instruction() {
        let initial_pc = 0x100;
        let mem_addr_for_lw = 0x0A00;
        let value_in_memory = 0xabcd_ef01;
        let initial_t0_val = 0x12345678;

        // $s0 (16) = mem_addr_for_lw
        // $t0 (8)  = initial_t0_val, then destination of LW
        // $t1 (9)  = scratch register modified in delay slot
        // $t2 (10) = should get loaded value from $t0

        let instructions = [
            i_type_instr(0x23, 16, 8, 0),     // LW $t0, 0($s0)
            i_type_instr(0x09, 0, 9, 0x5a5a), // Delay Slot: ADDIU $t1, $0, 0x5a5a (unrelated to $t0)
            r_type_instr(0x21, 8, 0, 10, 0),  // ADDU $t2, $t0, $0 (Instruction L+2)
        ];

        let mut cpu = build(initial_pc, &instructions);
        cpu.registers[16] = mem_addr_for_lw;
        cpu.registers[8] = initial_t0_val;
        cpu.ram.write32(mem_addr_for_lw, value_in_memory);

        cpu.step(); // Execute LW $t0, ...
        // After this step, $t0 still initial_t0_val, pending_load is Some for $t0
        assert_eq!(
            cpu.registers[8], initial_t0_val,
            "Delay slot: $t0 still old value"
        );

        cpu.step(); // Execute ADDIU $t1 (Delay Slot)
        assert_eq!(
            cpu.registers[9], 0x5a5a,
            "Delay slot: $t1 updated correctly"
        );

        cpu.step(); // Execute ADDU $t2, $t0, $0 (Instruction L+2)
        assert_eq!(
            cpu.registers[8], value_in_memory,
            "After delay: $t0 updated to loaded value"
        );
        assert_eq!(
            cpu.registers[10], value_in_memory,
            "After delay: $t2 gets new $t0 value"
        );
    }

    // Test 4: Delay slot is discarded if a following instruction overwrites the destination register.
    #[test]
    fn lw_delay_slot_overwritten() {
        let initial_pc = 0x100;
        let mem_addr_for_lw = 0x0A00;
        let value_in_memory = 0xabcd_ef01;
        let initial_t0_val = 0x12345678;

        // $s0 (16) = mem_addr_for_lw
        // $t0 (8)  = initial_t0_val, then destination of LW
        // $t1 (9)  = scratch register modified in delay slot
        // $t2 (10) = should get loaded value from $t0

        let instructions = [
            i_type_instr(0x23, 16, 8, 0),     // LW $t0, 0($s0)
            i_type_instr(0x09, 0, 8, 0x5a5a), // Delay Slot: ADDIU $t0, $0, 0x5a5a
            r_type_instr(0x21, 8, 0, 10, 0),  // ADDU $t2, $t0, $0 (Instruction L+2)
        ];

        let mut cpu = build(initial_pc, &instructions);
        cpu.registers[16] = mem_addr_for_lw;
        cpu.registers[8] = initial_t0_val;
        cpu.ram.write32(mem_addr_for_lw, value_in_memory);

        cpu.step(); // Execute LW $t0, ...
        // After this step, $t0 still initial_t0_val, pending_load is Some for $t0
        assert_eq!(
            cpu.registers[8], initial_t0_val,
            "Delay slot: $t0 still old value"
        );

        cpu.step(); // Execute ADDIU $t0 (Delay Slot)
        assert_eq!(
            cpu.registers[8], 0x5a5a,
            "Delay slot: $t1 updated correctly"
        );

        cpu.step(); // Execute ADDU $t2, $t0, $0 (Instruction L+2)
        assert_eq!(
            cpu.registers[8], 0x5a5a,
            "After delay: $t0 updated to loaded value"
        );
        assert_eq!(
            cpu.registers[10], 0x5a5a,
            "After delay: $t2 gets new $t0 value"
        );
    }

    // Test 5: Two consecutive LW instructions. Both should load correctly, each
    // into its own destination register, and with its own delay.
    #[test]
    fn consecutive_lw() {
        let initial_pc = 0x100;
        let mem_addr_for_lw1 = 0x0A00;
        let mem_addr_for_lw2 = 0x0A04;
        let value_in_memory1 = 0xabcd_ef01;
        let value_in_memory2 = 0x1234_5678;

        // $s0 (16) = mem_addr_for_lw1
        // $t0 (8)  = destination of LW1
        // $t1 (9)  = destination of LW2
        // $t2 (10) = should get loaded value from $t0

        let instructions = [
            i_type_instr(0x23, 16, 8, 0),    // LW $t0, 0($s0)
            i_type_instr(0x23, 16, 9, 4),    // LW $t1, 4($s0)
            r_type_instr(0x21, 8, 9, 10, 0), // ADDU $t2, $t0, $t1 (Instruction L+2)
        ];

        let mut cpu = build(initial_pc, &instructions);
        cpu.registers[8] = 0xcafecafe;
        cpu.registers[9] = 0xcafecafe;
        cpu.registers[16] = mem_addr_for_lw1;
        cpu.ram.write32(mem_addr_for_lw1, value_in_memory1);
        cpu.ram.write32(mem_addr_for_lw2, value_in_memory2);

        cpu.step(); // Execute LW $t0, ...
        assert_eq!(
            cpu.registers[8], 0xcafecafe,
            "Right after LW1: $t0 should be unchanged"
        );

        cpu.step(); // Execute LW $t1
        assert_eq!(
            cpu.registers[8], value_in_memory1,
            "After LW1: $t0 should be updated to loaded value"
        );
        assert_eq!(
            cpu.registers[9], 0xcafecafe,
            "Right after LW2: $t1 should be unchanged"
        );

        cpu.step(); // Execute ADDU $t2
        assert_eq!(
            cpu.registers[8], value_in_memory1,
            "After LW2: $t0 should be loaded value"
        );
        assert_eq!(
            cpu.registers[9], value_in_memory2,
            "After LW2: $t1 should be loaded value"
        );
        assert_eq!(
            cpu.registers[10],
            value_in_memory1.wrapping_add(0xcafecafe),
            "After LW2: $t2 should be sum of loaded $t0 and original $t1"
        );
    }
}
