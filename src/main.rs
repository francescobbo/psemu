mod cpu;
mod ram;

fn main() {
    let mut cpu = cpu::Cpu::new();
    cpu.write_memory(0, 0x2508_0001, 4);
    cpu.write_memory(4, 0x2508_0001, 4);
    cpu.write_memory(8, 0x2508_0001, 4);

    for _ in 0..3 {
        cpu.step();
    }

    // r8 should be 3
    println!("Registers: {:?}", cpu.registers);
}
