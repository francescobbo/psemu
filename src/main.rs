mod cpu;
mod ram;

use cpu::{AccessSize, Cpu};

//[ main
fn main() {
    let mut cpu = Cpu::new();

    // Write a test program to memory 
    cpu.write_memory(0, 0x2401_0002, AccessSize::Word);
    cpu.write_memory(4, 0x2421_0003, AccessSize::Word);

    // Execute the program
    for _ in 0..2 {
        cpu.step();
    }

    println!("r1: {}", cpu.registers[1]);
}
//] main
