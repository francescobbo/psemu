mod bus;
mod cpu;
mod debug;
mod emulator;
mod ram;

use bus::AccessSize;
use clap::Parser;
use emulator::Emulator;
use std::io::Read;

#[derive(Parser)]
struct MainArguments {
    /// Path to a PlayStation program
    #[arg(value_name = "PATH")]
    executable: Option<String>,

    /// Turn debugging information on
    #[arg(short, long)]
    debug: bool,
}

fn main() {
    // Parse command line arguments
    let args = MainArguments::parse();

    // Require a file to run
    if args.executable.is_none() {
        println!("No program file provided.");
        return;
    }

    // Create a new emulator instance
    let mut emulator = Emulator::new();

    // Set the debugger to be active if the user requested it
    emulator.debugger.stepping = args.debug;

    // Load the executable into the emulator's RAM
    let rom = read_rom(&args.executable.unwrap());
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
