use crate::{
    cpu::{Cpu, Instruction},
    ram::AccessSize,
};
use std::io::Write;

#[derive(Debug)]
pub struct Debugger {
    /// Flag to indicate if the debugger is in stepping mode
    pub stepping: bool,

    /// And instance of the disassembler, with its settings
    disasm: psdisasm::Disasm,
}

const REGISTERS: [&str; 32] = [
    "$zero", "$at", "$v0", "$v1", "$a0", "$a1", "$a2", "$a3", "$t0", "$t1", "$t2", "$t3", "$t4",
    "$t5", "$t6", "$t7", "$s0", "$s1", "$s2", "$s3", "$s4", "$s5", "$s6", "$s7", "$t8", "$t9",
    "$k0", "$k1", "$gp", "$sp", "$fp", "$ra",
];

impl Debugger {
    /// Create a new debugger instance
    pub fn new() -> Self {
        Debugger {
            stepping: false,
            disasm: psdisasm::Disasm::default(),
        }
    }

    /// Enter the debugger
    pub fn enter(&mut self, cpu: &mut Cpu) -> bool {
        // Present the current instruction
        let ins = cpu.read_memory(cpu.pc, AccessSize::Word).unwrap();
        println!("[{:08x}] {}", cpu.pc, self.disasm.disasm(ins, cpu.pc));

        loop {
            // Read a command from the user
            let line = self.read_line();

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
                "s" | "step" => {
                    break;
                }
                // Stop stepping
                "c" | "continue" => {
                    self.stepping = false;
                    break;
                }
                // No command, just continue
                "" => {}
                _ => println!("Unknown command: {}", cmd),
            }
        }

        return false
    }

    /// Read a line from the user
    pub fn read_line(&mut self) -> String {
        let mut line = String::new();

        // Print the prompt
        print!("> ");
        std::io::stdout().flush().unwrap();

        // Read a line from stdin
        std::io::stdin().read_line(&mut line).unwrap();

        // Remove trailing spaces and newlines
        line.trim().to_string()
    }

    /// Prints the contents of the registers
    pub fn print_registers(cpu: &Cpu) {
        for (i, &value) in cpu.registers.iter().enumerate() {
            print!("{:>5} -> {value:08x}  ", REGISTERS[i]);

            if i % 4 == 3 {
                println!();
            }
        }

        println!("   pc -> {:08x}", cpu.pc);
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
