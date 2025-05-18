mod cpu;
mod ram;

fn main() {
    let mut cpu = cpu::Cpu::new();
    cpu.ram.write32(0, 0x2508_0001);
    cpu.ram.write32(4, 0x2508_0001);
    cpu.ram.write32(8, 0x2508_0001);

    for _ in 0..3 {
        cpu.step();
    }

    // r8 should be 3
    println!("Registers: {:?}", cpu.registers);
}
