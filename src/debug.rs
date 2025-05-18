use crate::{
    cpu::Cpu,
    ram::AccessSize,
};

pub struct Debugger {
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
}
