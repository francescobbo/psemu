//[ mod-cpu
mod cpu;
//[ !omit
//[ mod-ram
mod ram;
//] mod-ram
//] !omit

use cpu::{AccessSize, Cpu};
//] mod-cpu

//[ main
fn main() {
    let mut cpu = Cpu::new();

    // Write some invalid instructions to the RAM, through the CPU's memory
    // interface
    cpu.write_memory(0, 0x12345678, AccessSize::Word);
    cpu.write_memory(4, 0x87654321, AccessSize::Word);
    cpu.write_memory(8, 0xdeadbeef, AccessSize::Word);

    for _ in 0..3 {
        cpu.step(); // Execute the instructions
    }
}
//] main
