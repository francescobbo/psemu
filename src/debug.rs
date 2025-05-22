use crate::{
    cpu::{Cpu, Instruction},
    ram::AccessSize,
};

pub struct Debugger {}

/// Represents the different combinations of arguments for the opcodes
#[allow(non_camel_case_types)]
enum ArgumentTypes {
    D_S_T,     // rd, rs, rt
    D_T_S,     // rd, rt, rs
    D_T_Shift, // rt, rd, shift
    T_S_SImm,  // rt, rs, sign_extend_imm
    T_Imm,    // rt, imm
    T_S_Imm,   // rt, rs, imm
    T_Mem,     // rt, offset(rs)
}

impl Debugger {
    /// Prints the contents of the registers
    pub fn print_registers(cpu: &Cpu) {
        for (i, &value) in cpu.registers.iter().enumerate() {
            print!("r{i:<2}: {value:08x}  ");

            if i % 4 == 3 {
                println!();
            }
        }

        println!("pc: {:#08x}", cpu.pc);
    }

    /// Prints the contents of a memory location, as a little endian 32-bit
    /// integer
    pub fn read_memory(cpu: &Cpu, address: u32) {
        match cpu.read_memory(address, AccessSize::Word) {
            Ok(value) => println!("{address:08x}: {value:08x}"),
            Err(_) => println!("Error reading memory at address {address:08x}"),
        }
    }

    /// Disassemble an instruction
    pub fn disassemble(ins: Instruction) -> String {
        match ins.opcode() {
            0x00 => match ins.funct() {
                0x00 => Self::format_instruction(ins, "sll", ArgumentTypes::D_T_Shift),
                0x02 => Self::format_instruction(ins, "srl", ArgumentTypes::D_T_Shift),
                0x03 => Self::format_instruction(ins, "sra", ArgumentTypes::D_T_Shift),
                0x04 => Self::format_instruction(ins, "sllv", ArgumentTypes::D_T_S),
                0x06 => Self::format_instruction(ins, "srlv", ArgumentTypes::D_T_S),
                0x07 => Self::format_instruction(ins, "srav", ArgumentTypes::D_T_S),
                0x20 => Self::format_instruction(ins, "add", ArgumentTypes::D_S_T),
                0x21 => Self::format_instruction(ins, "addu", ArgumentTypes::D_S_T),
                0x22 => Self::format_instruction(ins, "sub", ArgumentTypes::D_S_T),
                0x23 => Self::format_instruction(ins, "subu", ArgumentTypes::D_S_T),
                0x24 => Self::format_instruction(ins, "and", ArgumentTypes::D_S_T),
                0x25 => Self::format_instruction(ins, "or", ArgumentTypes::D_S_T),
                0x26 => Self::format_instruction(ins, "xor", ArgumentTypes::D_S_T),
                0x27 => Self::format_instruction(ins, "nor", ArgumentTypes::D_S_T),
                0x2a => Self::format_instruction(ins, "slt", ArgumentTypes::D_S_T),
                0x2b => Self::format_instruction(ins, "sltu", ArgumentTypes::D_S_T),
                _ => format!("Invalid opcode 0x00 with funct {:x}", ins.funct()),
            },
            0x08 => Self::format_instruction(ins, "addi", ArgumentTypes::T_S_SImm),
            0x09 => Self::format_instruction(ins, "addiu", ArgumentTypes::T_S_SImm),
            0x0a => Self::format_instruction(ins, "slti", ArgumentTypes::T_S_SImm),
            0x0b => Self::format_instruction(ins, "sltiu", ArgumentTypes::T_S_Imm),
            0x0c => Self::format_instruction(ins, "andi", ArgumentTypes::T_S_Imm),
            0x0d => Self::format_instruction(ins, "ori", ArgumentTypes::T_S_Imm),
            0x0e => Self::format_instruction(ins, "xori", ArgumentTypes::T_S_Imm),
            0x0f => Self::format_instruction(ins, "lui", ArgumentTypes::T_Imm),
            0x20 => Self::format_instruction(ins, "lb", ArgumentTypes::T_Mem),
            0x21 => Self::format_instruction(ins, "lh", ArgumentTypes::T_Mem),
            0x23 => Self::format_instruction(ins, "lw", ArgumentTypes::T_Mem),
            0x24 => Self::format_instruction(ins, "lbu", ArgumentTypes::T_Mem),
            0x25 => Self::format_instruction(ins, "lhu", ArgumentTypes::T_Mem),
            0x28 => Self::format_instruction(ins, "sb", ArgumentTypes::T_Mem),
            0x29 => Self::format_instruction(ins, "sh", ArgumentTypes::T_Mem),
            0x2b => Self::format_instruction(ins, "sw", ArgumentTypes::T_Mem),
            _ => format!("Unknown opcode: {:#x}", ins.opcode()),
        }
    }

    /// Format the instruction with its name and arguments
    fn format_instruction(ins: Instruction, name: &str, arg_types: ArgumentTypes) -> String {
        let args = Self::format_args(ins, arg_types);
        format!("{name} {args}")
    }

    /// Format the arguments types for display
    fn format_args(ins: Instruction, arg_types: ArgumentTypes) -> String {
        match arg_types {
            ArgumentTypes::D_S_T => format!("r{}, r{}, r{}", ins.rd(), ins.rs(), ins.rt(),),
            ArgumentTypes::D_T_S => format!("r{}, r{}, r{}", ins.rd(), ins.rt(), ins.rs()),
            ArgumentTypes::D_T_Shift => format!("r{}, r{}, {}", ins.rd(), ins.rt(), ins.shamt()),
            ArgumentTypes::T_S_SImm => format!("r{}, r{}, {:#x}", ins.rt(), ins.rs(), ins.simm16()),
            ArgumentTypes::T_Imm => format!("r{}, {:#x}", ins.rt(), ins.imm16()),
            ArgumentTypes::T_S_Imm => format!("r{}, r{}, {:#x}", ins.rt(), ins.rs(), ins.imm16()),
            ArgumentTypes::T_Mem => format!("r{}, {}(r{})", ins.rt(), ins.simm16(), ins.rs()),
        }
    }
}
