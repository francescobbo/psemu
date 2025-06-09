//[ new-file
use crate::{AccessSize, Cpu};

#[derive(Default)]
pub struct Debugger {}

//[ display-mode
/// Determines how the values are displayed in the debugger
#[derive(Debug)]
pub enum DisplayMode {
    Binary,
    Octal,
    Decimal,
    Hexadecimal,
    Character,
}
//] display-mode

impl Debugger {
    //[ print-registers
    /// Prints the contents of the CPU registers
    pub fn print_registers(&self, cpu: &Cpu) {
        for (i, &value) in cpu.registers.iter().enumerate() {
            print!("r{i:<2}: {value:08x}  ");

            if i % 4 == 3 {
                println!();
            }
        }

        println!("pc: {:#08x}", cpu.pc);
    }
    //] print-registers

    //[ read-memory
    /// Prints the contents of a memory location (of the specified size),
    /// considering a little-endian format.
    pub fn read_memory(
        &self,
        cpu: &Cpu,
        address: u32,
        mode: DisplayMode,
        size: AccessSize,
    ) {
        print!("{address:08x}: ");

        match cpu.read_memory(address, size) {
            Ok(value) => println!("{}", self.format_value(value, mode)),
            Err(_) => println!("Error reading memory"),
        }
    }

    //[ format-value
    fn format_value(&self, value: u32, mode: DisplayMode) -> String {
        match mode {
            DisplayMode::Binary => format!("{value:032b}"),
            DisplayMode::Octal => format!("{value:o}"),
            DisplayMode::Decimal => value.to_string(),
            DisplayMode::Hexadecimal => format!("{value:08x}"),
            DisplayMode::Character => format!("{}", value as u8 as char),
        }
    }
    //] format-value
    //] read-memory
}
//] new-file
