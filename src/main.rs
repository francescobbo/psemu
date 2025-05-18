mod cpu;
mod debug;
mod ram;

use std::io::Read;
use debug::Debugger;

fn main() {
    let mut cpu = cpu::Cpu::new();

    // If there's a cmd line argument, treat it as a file path
    // and load the program into RAM.
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("No program file provided.");
        return;
    }

    let rom = read_rom(&args[1]);
    load_rom(&mut cpu, rom, 0x1000);
    cpu.pc = 0x1000;

    // Execute 100 instructions
    for _ in 0..42 {
        cpu.step();
    }

    // Print the contents of the registers
    Debugger::print_registers(&cpu);

    // Print the contents of the RAM 0x100 to 0x116
    for address in (0x100..=0x116).step_by(4) {
        Debugger::read_memory(&cpu, address);
    }
}

fn read_rom(path: &str) -> Vec<u8> {
    let mut file = std::fs::File::open(path).expect("Failed to open file");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Failed to read file");
    buffer
}

fn load_rom(cpu: &mut cpu::Cpu, rom: Vec<u8>, start_address: u32) {
    for (i, byte) in rom.iter().enumerate() {
        cpu.write_memory(start_address + i as u32, *byte as u32, ram::AccessSize::Byte)
            .expect("Failed to write to memory");
    }
}
