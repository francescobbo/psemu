//[ !omit
#![allow(dead_code)]
use super::{AccessSize, Cpu};

/// Creates a new `Cpu` instance with the specified initial registers and
/// instructions.
///
/// The PC is set to 0x1000 and the instructions are written to the memory
/// starting from that address.
pub fn test_cpu(
    initial_registers: &[(usize, u32)],
    instructions: &[u32],
) -> Cpu {
    let mut cpu = Cpu::new();
    cpu.pc = 0x1000;

    for &(index, value) in initial_registers.iter() {
        if index < cpu.registers.len() {
            cpu.registers[index] = value;
        } else {
            panic!("Register index out of bounds: {index}");
        }
    }

    for (i, &instr_word) in instructions.iter().enumerate() {
        cpu.write_memory(cpu.pc + (i as u32 * 4), instr_word, AccessSize::Word)
            .expect("Failed to write instruction to bus during test setup");
    }

    cpu
}

/// Advances the CPU by a specified number of steps.
pub fn cpu_steps(cpu: &mut Cpu, steps: usize) {
    for _ in 0..steps {
        cpu.step();
    }
}

/// Encodes a MIPS I-Type instruction.
pub fn i_type(opcode: u32, rt: usize, rs: usize, immediate: u16) -> u32 {
    (opcode << 26)
        | ((rs as u32) << 21)
        | ((rt as u32) << 16)
        | (immediate as u32)
}

/// Encodes a MIPS R-Type instruction with a shift amount.
pub fn r_type_shift(funct: u32, rd: usize, rt: usize, shamt: usize) -> u32 {
    // Assuming opcode for SPECIAL R-Type is 0x00
    ((rt as u32) << 16) | ((rd as u32) << 11) | ((shamt as u32) << 6) | funct
}

/// Encodes a MIPS R-Type instruction with a three-register format.
pub fn r_type(funct: u32, rd: usize, rt: usize, rs: usize) -> u32 {
    // Assuming opcode for SPECIAL R-Type is 0x00
    ((rs as u32) << 21) | ((rt as u32) << 16) | ((rd as u32) << 11) | funct
}

/// Encodes a MIPS J-Type instruction.
pub fn j_type(opcode: u32, target_pseudo_addr: u32) -> u32 {
    (opcode << 26) | (target_pseudo_addr >> 2)
}
//] !omit
