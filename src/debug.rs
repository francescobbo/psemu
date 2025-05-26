use crate::{
    bus::AccessSize,
    cpu::{Cpu, Instruction},
};
use rustyline::{DefaultEditor, error::ReadlineError};

#[derive(Debug)]
pub struct Debugger {
    /// Flag to indicate if the debugger is in stepping mode
    pub stepping: bool,

    /// Rustyline instance for command line input, with no special configuration.
    editor: DefaultEditor,
}

/// Represents the different combinations of arguments for the opcodes
#[allow(non_camel_case_types)]
enum ArgumentTypes {
    D_S_T,     // rd, rs, rt
    D_T_S,     // rd, rt, rs
    D_T_Shift, // rt, rd, shift
    T_S_SImm,  // rt, rs, sign_extend_imm
    T_Imm,     // rt, imm
    T_S_Imm,   // rt, rs, imm
    T_Mem,     // rt, offset(rs)
    D,         // rd
    S,         // rs
    S_D,       // rs, rd
    S_T,       // rs, rt
    S_T_Jump,  // rs, rt, target
    S_Jump,    // rs, target
    Jump,      // target
}

const REGISTERS: [&str; 32] = [
    "$zero", "$at", "$v0", "$v1", "$a0", "$a1", "$a2", "$a3", "$t0", "$t1", "$t2", "$t3", "$t4",
    "$t5", "$t6", "$t7", "$s0", "$s1", "$s2", "$s3", "$s4", "$s5", "$s6", "$s7", "$t8", "$t9",
    "$k0", "$k1", "$gp", "$sp", "$fp", "$ra",
];

const HISTORY_FILE: &str = ".dbg_history";

impl Debugger {
    /// Create a new debugger instance
    pub fn new() -> Self {
        let mut editor = DefaultEditor::new().unwrap();
        let _ = editor.load_history(HISTORY_FILE);

        Debugger {
            stepping: false,
            editor,
        }
    }

    /// Enter the debugger
    pub fn enter(&mut self, cpu: &mut Cpu) -> bool {
        // Present the current instruction
        let ins = cpu.read_memory(cpu.pc, AccessSize::Word).unwrap();
        println!(
            "[{:08x}]    {}",
            cpu.pc,
            Self::disassemble(Instruction(ins), cpu.pc.wrapping_add(4))
        );

        loop {
            // Read a command from the user. Return true if this is None
            // (e.g. the user pressed Ctrl-C)
            let Some(line) = self.read_line() else {
                return true;
            };

            // Take the first word as the command
            let mut parts = line.split_whitespace();
            let cmd = parts.next().unwrap_or("");

            match cmd {
                // Quit the debugger
                "q" | "quit" => {
                    println!("Quitting...");
                    return true;
                }
                // Step the CPU
                "s" | "step" | "" => {
                    break;
                }
                // Stop stepping
                "c" | "continue" => {
                    self.stepping = false;
                    break;
                }
                // Show the registers
                "r" | "registers" => {
                    Self::print_registers(cpu);
                }
                // Read memory
                "rm" | "read-mem" => {
                    // Get the address from the command line
                    let Some(address_str) = parts.next() else {
                        println!("Usage: read-mem <address>");
                        continue;
                    };

                    // Parse the address
                    let Ok(address) = Self::parse_hex(address_str) else {
                        println!("Invalid address: {address_str}");
                        continue;
                    };

                    Self::read_memory(cpu, address);
                }
                _ => println!("Unknown command: {}", cmd),
            }
        }

        return false
    }

    /// Parses a string as a hexadecimal number, allowing for an optional "0x" prefix.
    fn parse_hex(string: &str) -> Result<u32, std::num::ParseIntError> {
        let string = string.strip_prefix("0x").unwrap_or(string);

        u32::from_str_radix(string, 16)
    }

    /// Read a line from the user
    pub fn read_line(&mut self) -> Option<String> {
        match self.editor.readline("> ") {
            Ok(line) => {
                // Add the line to the history
                let line = line.trim().to_string();

                // Add the line to the history
                let _ = self.editor.add_history_entry(&line);

                Some(line)
            }
            Err(ReadlineError::Interrupted) => {
                None
            }
            Err(_) => {
                println!("Error reading line");
                Some(String::new())
            }
        }
    }

    /// Prints the contents of the registers
    pub fn print_registers(cpu: &Cpu) {
        for (i, &value) in cpu.registers.iter().enumerate() {
            print!("{:>5} -> {value:08x}  ", REGISTERS[i]);

            if i % 4 == 3 {
                println!();
            }
        }

        println!(
            "   pc -> {:08x}     hi -> {:08x}     lo -> {:08x}",
            cpu.pc, cpu.hi, cpu.lo
        );

        if let Some(load_delay) = &cpu.load_delay {
            println!(
                "Pending load: {} -> {:08x}",
                REGISTERS[load_delay.target], load_delay.value
            );
        }
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
    pub fn disassemble(ins: Instruction, pc: u32) -> String {
        match ins.opcode() {
            0x00 => match ins.funct() {
                0x00 => Self::format_instruction(ins, "sll", ArgumentTypes::D_T_Shift, pc),
                0x02 => Self::format_instruction(ins, "srl", ArgumentTypes::D_T_Shift, pc),
                0x03 => Self::format_instruction(ins, "sra", ArgumentTypes::D_T_Shift, pc),
                0x04 => Self::format_instruction(ins, "sllv", ArgumentTypes::D_T_S, pc),
                0x06 => Self::format_instruction(ins, "srlv", ArgumentTypes::D_T_S, pc),
                0x07 => Self::format_instruction(ins, "srav", ArgumentTypes::D_T_S, pc),
                0x08 => Self::format_instruction(ins, "jr", ArgumentTypes::S, pc),
                0x09 => Self::format_instruction(ins, "jalr", ArgumentTypes::S_D, pc),
                0x10 => Self::format_instruction(ins, "mfhi", ArgumentTypes::D, pc),
                0x11 => Self::format_instruction(ins, "mthi", ArgumentTypes::S, pc),
                0x12 => Self::format_instruction(ins, "mflo", ArgumentTypes::D, pc),
                0x13 => Self::format_instruction(ins, "mtlo", ArgumentTypes::S, pc),
                0x18 => Self::format_instruction(ins, "mult", ArgumentTypes::S_T, pc),
                0x19 => Self::format_instruction(ins, "multu", ArgumentTypes::S_T, pc),
                0x1a => Self::format_instruction(ins, "div", ArgumentTypes::S_T, pc),
                0x1b => Self::format_instruction(ins, "divu", ArgumentTypes::S_T, pc),
                0x20 => Self::format_instruction(ins, "add", ArgumentTypes::D_S_T, pc),
                0x21 => Self::format_instruction(ins, "addu", ArgumentTypes::D_S_T, pc),
                0x22 => Self::format_instruction(ins, "sub", ArgumentTypes::D_S_T, pc),
                0x23 => Self::format_instruction(ins, "subu", ArgumentTypes::D_S_T, pc),
                0x24 => Self::format_instruction(ins, "and", ArgumentTypes::D_S_T, pc),
                0x25 => Self::format_instruction(ins, "or", ArgumentTypes::D_S_T, pc),
                0x26 => Self::format_instruction(ins, "xor", ArgumentTypes::D_S_T, pc),
                0x27 => Self::format_instruction(ins, "nor", ArgumentTypes::D_S_T, pc),
                0x2a => Self::format_instruction(ins, "slt", ArgumentTypes::D_S_T, pc),
                0x2b => Self::format_instruction(ins, "sltu", ArgumentTypes::D_S_T, pc),
                _ => format!("Invalid opcode 0x00 with funct {:x}", ins.funct()),
            },
            0x01 => {
                // This format abuses the `rt` field for a sub-opcode
                match ins.rt() {
                    0x00 => Self::format_instruction(ins, "bltz", ArgumentTypes::S_Jump, pc),
                    0x01 => Self::format_instruction(ins, "bgez", ArgumentTypes::S_Jump, pc),
                    0x10 => Self::format_instruction(ins, "bltzal", ArgumentTypes::S_Jump, pc),
                    0x11 => Self::format_instruction(ins, "bgezal", ArgumentTypes::S_Jump, pc),
                    _ => panic!("Invalid opcode 0x01 with rt {:x}", ins.rt()),
                }
            }
            0x02 => Self::format_instruction(ins, "j", ArgumentTypes::Jump, pc),
            0x03 => Self::format_instruction(ins, "jal", ArgumentTypes::Jump, pc),
            0x04 => Self::format_instruction(ins, "beq", ArgumentTypes::S_T_Jump, pc),
            0x05 => Self::format_instruction(ins, "bne", ArgumentTypes::S_T_Jump, pc),
            0x06 => Self::format_instruction(ins, "blez", ArgumentTypes::S_Jump, pc),
            0x07 => Self::format_instruction(ins, "bgtz", ArgumentTypes::S_Jump, pc),
            0x08 => Self::format_instruction(ins, "addi", ArgumentTypes::T_S_SImm, pc),
            0x09 => Self::format_instruction(ins, "addiu", ArgumentTypes::T_S_SImm, pc),
            0x0a => Self::format_instruction(ins, "slti", ArgumentTypes::T_S_SImm, pc),
            0x0b => Self::format_instruction(ins, "sltiu", ArgumentTypes::T_S_Imm, pc),
            0x0c => Self::format_instruction(ins, "andi", ArgumentTypes::T_S_Imm, pc),
            0x0d => Self::format_instruction(ins, "ori", ArgumentTypes::T_S_Imm, pc),
            0x0e => Self::format_instruction(ins, "xori", ArgumentTypes::T_S_Imm, pc),
            0x0f => Self::format_instruction(ins, "lui", ArgumentTypes::T_Imm, pc),
            0x10..=0x13 => Self::format_coprocessor_instruction(ins),
            0x20 => Self::format_instruction(ins, "lb", ArgumentTypes::T_Mem, pc),
            0x21 => Self::format_instruction(ins, "lh", ArgumentTypes::T_Mem, pc),
            0x22 => Self::format_instruction(ins, "lwl", ArgumentTypes::T_Mem, pc),
            0x23 => Self::format_instruction(ins, "lw", ArgumentTypes::T_Mem, pc),
            0x24 => Self::format_instruction(ins, "lbu", ArgumentTypes::T_Mem, pc),
            0x25 => Self::format_instruction(ins, "lhu", ArgumentTypes::T_Mem, pc),
            0x26 => Self::format_instruction(ins, "lwr", ArgumentTypes::T_Mem, pc),
            0x28 => Self::format_instruction(ins, "sb", ArgumentTypes::T_Mem, pc),
            0x29 => Self::format_instruction(ins, "sh", ArgumentTypes::T_Mem, pc),
            0x2a => Self::format_instruction(ins, "swl", ArgumentTypes::T_Mem, pc),
            0x2b => Self::format_instruction(ins, "sw", ArgumentTypes::T_Mem, pc),
            0x2e => Self::format_instruction(ins, "swr", ArgumentTypes::T_Mem, pc),
            _ => format!("Unknown opcode: {:#x}", ins.opcode()),
        }
    }

    /// Format the instruction with its name and arguments
    fn format_instruction(
        ins: Instruction,
        name: &str,
        arg_types: ArgumentTypes,
        pc: u32,
    ) -> String {
        let args = Self::format_args(ins, arg_types, pc);
        format!("{name} {args}")
    }

    /// Format the arguments types for display
    fn format_args(ins: Instruction, arg_types: ArgumentTypes, pc: u32) -> String {
        match arg_types {
            ArgumentTypes::D_S_T => format!(
                "{}, {}, {}",
                REGISTERS[ins.rd()],
                REGISTERS[ins.rs()],
                REGISTERS[ins.rt()]
            ),
            ArgumentTypes::D_T_S => format!(
                "{}, {}, {}",
                REGISTERS[ins.rd()],
                REGISTERS[ins.rt()],
                REGISTERS[ins.rs()]
            ),
            ArgumentTypes::D_T_Shift => format!(
                "{}, {}, {}",
                REGISTERS[ins.rd()],
                REGISTERS[ins.rt()],
                ins.shamt()
            ),
            ArgumentTypes::T_S_SImm => format!(
                "{}, {}, {:#x}",
                REGISTERS[ins.rt()],
                REGISTERS[ins.rs()],
                ins.simm16()
            ),
            ArgumentTypes::T_Imm => format!("{}, {:#x}", REGISTERS[ins.rt()], ins.imm16()),
            ArgumentTypes::T_S_Imm => format!(
                "{}, {}, {:#x}",
                REGISTERS[ins.rt()],
                REGISTERS[ins.rs()],
                ins.imm16()
            ),
            ArgumentTypes::T_Mem => format!(
                "{}, {}({})",
                REGISTERS[ins.rt()],
                ins.simm16(),
                REGISTERS[ins.rs()]
            ),
            ArgumentTypes::S => format!("{}", REGISTERS[ins.rs()]),
            ArgumentTypes::D => format!("{}", REGISTERS[ins.rd()]),
            ArgumentTypes::S_D => format!("{}, {}", REGISTERS[ins.rs()], REGISTERS[ins.rd()]),
            ArgumentTypes::S_T => format!("{}, {}", REGISTERS[ins.rs()], REGISTERS[ins.rt()]),
            ArgumentTypes::S_T_Jump => {
                let target = pc.wrapping_add((ins.simm16() << 2) as u32);

                format!(
                    "{}, {}, {:x}",
                    REGISTERS[ins.rs()],
                    REGISTERS[ins.rt()],
                    target
                )
            }
            ArgumentTypes::S_Jump => {
                let target = pc.wrapping_add((ins.simm16() << 2) as u32);
                format!("{}, {:x}", REGISTERS[ins.rs()], target)
            }
            ArgumentTypes::Jump => {
                let target = (pc & 0xf000_0000) | (ins.jump_target() << 2);
                format!("{:x}", target)
            }
        }
    }

    /// Coprocessor instruction have unique formatting rules. This function formats
    /// them based on the opcode and the specific coprocessor instruction.
    fn format_coprocessor_instruction(ins: Instruction) -> String {
        // Get the coprocessor number from the opcode
        let cop = ins.opcode() & 3;

        if ins.cop_execute() {
            // Coprocessor specific opcode
            format!("cop{} execute: {:x}", cop, ins.0 & 0xffffff)
        } else {
            let gpr = REGISTERS[ins.rt()];
            let cop_reg = ins.rd();

            match ins.cop_funct() {
                0 => format!("mfc{cop} {gpr}, cop{cop_reg}"),
                2 => format!("cfc{cop} {gpr}, cop{}", cop_reg + 32),
                4 => format!("mtc{cop} cop{cop_reg}, {gpr}"),
                6 => format!("ctc{cop} cop{}, {gpr}", cop_reg + 32),
                _ => format!("unknown cop{} funct: {:x}", cop, ins.cop_funct()),
            }
        }
    }
}

impl Drop for Debugger {
    fn drop(&mut self) {
        // Save the history to a file
        let _ = self.editor.save_history(HISTORY_FILE);
    }
}
