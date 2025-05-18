use ram::AccessSize;

mod cpu;
mod ram;

fn main() {
    let mut cpu = cpu::Cpu::new();
    cpu.write_memory(0, 0x2508_0001, AccessSize::Word);
    cpu.write_memory(4, 0x2508_0001, AccessSize::Word);
    cpu.write_memory(8, 0x2508_0001, AccessSize::Word);

    for _ in 0..3 {
        cpu.step();
    }

    // r8 should be 3
    println!("Registers: {:?}", cpu.registers);
}
