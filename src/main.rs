mod cpu;
mod ram;

use std::io::Read;

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
    load_rom(&mut cpu.ram, rom, 0x1000);
    cpu.pc = 0x1000;

    // Execute 100 instructions
    for _ in 0..42 {
        cpu.step();
    }

    // Print the contents of the registers
    for (i, &reg) in cpu.registers.iter().enumerate() {
        print!("r{:<2}: 0x{:08x}  ", i, reg);

        if i % 4 == 3 {
            println!();
        }
    }

    println!();

    // Print the contents of the RAM 0x100 to 0x116
    for i in 0..=5 {
        let address = 0x100 + i * 4;
        let value = cpu.ram.read32(address);
        println!("0x{:08x}: 0x{:08x}", address, value);
    }
}

fn read_rom(path: &str) -> Vec<u8> {
    let mut file = std::fs::File::open(path).expect("Failed to open file");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Failed to read file");
    buffer
}

fn load_rom(ram: &mut ram::Ram, rom: Vec<u8>, start_address: u32) {
    for (i, byte) in rom.iter().enumerate() {
        ram.write8(start_address + i as u32, *byte);
    }
}