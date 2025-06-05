use crate::{
    cpu::Cpu,
    bus::AccessSize,
};
use rustyline::{DefaultEditor, error::ReadlineError};
use std::collections::HashSet;

#[derive(Debug)]
pub struct Debugger {
    /// Flag to indicate if the debugger is in stepping mode
    pub stepping: bool,

    /// And instance of the disassembler, with its settings
    disasm: psdisasm::Disasm,

    /// Rustyline instance for command line input, with no special configuration.
    editor: DefaultEditor,

    /// The addresses where the debugger will break the execution
    breakpoints: HashSet<u32>,
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
            breakpoints: HashSet::new(),
        }
    }

    /// Enter the debugger
    pub fn enter(&mut self, cpu: &mut Cpu) -> bool {
        // Present the current instruction
        let ins = cpu.read_memory(cpu.pc, AccessSize::Word).unwrap();
        let is_branch_delay_slot = cpu.branch_target.is_some();

        println!(
            "[{:08x}] {}  {}",
            cpu.pc,
            if is_branch_delay_slot { "D" } else { " " },
            self.disasm.disasm_with_context(ins, cpu.pc.wrapping_add(4), &cpu.registers)
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
                // Add a breakpoint
                "b" | "breakpoint" => {
                    // Get the address from the command line
                    let Some(address_str) = parts.next() else {
                        println!("Usage: breakpoint <address>");
                        continue;
                    };

                    // Parse the address
                    let Ok(address) = Self::parse_hex(address_str) else {
                        println!("Invalid address: {address_str}");
                        continue;
                    };

                    // Add the breakpoint
                    self.breakpoints.insert(address);
                }
                // List breakpoints
                "bl" | "breakpoints" => {
                    if self.breakpoints.is_empty() {
                        println!("No breakpoints set.");
                    } else {
                        println!("Breakpoints:");
                        for &address in &self.breakpoints {
                            println!("  0x{:08x}", address);
                        }
                    }
                }
                // Remove a breakpoint
                "rb" | "remove-breakpoint" => {
                    // Get the address from the command line
                    let Some(address_str) = parts.next() else {
                        println!("Usage: remove-breakpoint <address>");
                        continue;
                    };

                    // Parse the address
                    let Ok(address) = Self::parse_hex(address_str) else {
                        println!("Invalid address: {address_str}");
                        continue;
                    };

                    // Remove the breakpoint
                    if self.breakpoints.remove(&address) {
                        println!("Breakpoint at {address:08x} removed.");
                    } else {
                        println!("No breakpoint at {address:08x}.");
                    }
                }
                _ => println!("Unknown command: {}", cmd),
            }
        }

        return false;
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
            Err(ReadlineError::Interrupted) => None,
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

    /// Checks if the given address is a breakpoint.
    pub fn has_breakpoint(&self, address: u32) -> bool {
        self.breakpoints.contains(&address)
    }
}

impl Drop for Debugger {
    fn drop(&mut self) {
        // Save the history to a file
        let _ = self.editor.save_history(HISTORY_FILE);
    }
}
