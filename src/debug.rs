use crate::{
    cpu::Cpu,
    ram::AccessSize,
};
use rustyline::{DefaultEditor, error::ReadlineError};

#[derive(Debug)]
pub struct Debugger {
    /// Flag to indicate if the debugger is in stepping mode
    pub stepping: bool,

    /// And instance of the disassembler, with its settings
    disasm: psdisasm::Disasm,

    /// Rustyline instance for command line input, with no special configuration.
    editor: DefaultEditor,
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
            disasm: psdisasm::Disasm::default(),
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
            self.disasm.disasm(ins, cpu.pc)
        );

        loop {
            // Read a command from the user
            let Some(line) = self.read_line() else {
                // If we couldn't read a line, just continue
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
}

impl Drop for Debugger {
    fn drop(&mut self) {
        println!("Saving history...");
        // Save the history to a file
        let _ = self.editor.save_history(HISTORY_FILE);
    }
}
