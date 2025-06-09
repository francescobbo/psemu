mod cpu;
//[ mod-debugger
mod debugger;
//] mod-debugger
mod ram;

use cpu::{AccessSize, Cpu};

//[ main
fn main() {
    let mut cpu = Cpu::new();

    // Write a test program to memory 
    cpu.write_memory(0, 0x2402_00fc, AccessSize::Word);
    cpu.write_memory(4, 0x8c43_0004, AccessSize::Word);
    cpu.write_memory(8, 0x2463_0001, AccessSize::Word);
    cpu.write_memory(12, 0xac43_0008, AccessSize::Word);

    // Write a value at 0x100
    cpu.write_memory(0x100, 41, AccessSize::Word);

    // Execute the program
    for _ in 0..4 {
        cpu.step();
    }

    // Read the result from memory, at 0x104
    let result = cpu.read_memory(0x104, AccessSize::Word).unwrap();
    println!("Result: {}", result);
}
//] main
