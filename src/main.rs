mod bus;
mod cpu;
mod debug;
mod emulator;
mod ram;

use bus::AccessSize;
use emulator::Emulator;
use std::io::Read;

fn main() {
    // If there's a cmd line argument, treat it as a file path
    // and load the program into RAM.
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("No program file provided.");
        return;
    }

    let mut emulator = Emulator::new();
    emulator.debugger.stepping = true;

    let rom = read_rom(&args[1]);
    load_rom(&mut emulator.cpu, rom, 0);

    // Run forever
    emulator.run();
}

fn read_rom(path: &str) -> Vec<u8> {
    let mut file = std::fs::File::open(path).expect("Failed to open file");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Failed to read file");
    buffer
}

fn load_rom(cpu: &mut cpu::Cpu, rom: Vec<u8>, start_address: u32) {
    for (i, byte) in rom.iter().enumerate() {
        cpu.write_memory(start_address + i as u32, *byte as u32, AccessSize::Byte)
            .expect("Failed to write to memory");
    }
}
