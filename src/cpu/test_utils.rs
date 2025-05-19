#![allow(dead_code)]
use crate::{cpu::Cpu, ram::AccessSize};

const INITIAL_PC: u32 = 0x0000_1000;

pub fn test_cpu(initial_registers: &[(usize, u32)], instructions: &[u32]) -> Cpu {
    let mut cpu = Cpu::new();
    cpu.pc = INITIAL_PC;

    for &(index, value) in initial_registers.iter() {
        if index < cpu.registers.len() {
            cpu.registers[index] = value;
        } else {
            panic!("Register index out of bounds: {index}");
        }
    }

    for (i, &instr_word) in instructions.iter().enumerate() {
        cpu.write_memory(INITIAL_PC + (i as u32 * 4), instr_word, AccessSize::Word)
            .expect("Failed to write instruction to bus during test setup");
    }

    cpu
}

pub fn cpu_steps(cpu: &mut Cpu, steps: usize) {
    for _ in 0..steps {
        cpu.step();
    }
}

pub fn i_type(opcode: u32, rt: usize, rs: usize, immediate: u16) -> u32 {
    (opcode << 26) | ((rs as u32) << 21) | ((rt as u32) << 16) | (immediate as u32)
}

pub fn r_type_shift(funct: u32, rd: usize, rt: usize, shamt: usize) -> u32 {
    // Assuming opcode for SPECIAL R-Type is 0x00
    ((rt as u32) << 16) | ((rd as u32) << 11) | ((shamt as u32) << 6) | funct
}

pub fn r_type(funct: u32, rd: usize, rt: usize, rs: usize) -> u32 {
    // Assuming opcode for SPECIAL R-Type is 0x00
    ((rs as u32) << 21) | ((rt as u32) << 16) | ((rd as u32) << 11) | funct
}
