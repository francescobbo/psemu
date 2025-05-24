mod bus;
mod cpu;
mod debug;
mod emulator;
mod executable;
mod ram;
mod rom;

use bus::AccessSize;
use clap::Parser;
use emulator::Emulator;
use executable::Executable;

#[derive(Parser)]
struct MainArguments {
    /// Path to a PlayStation program
    #[arg(value_name = "PATH")]
    executable: Option<String>,

    /// Path to a PlayStation BIOS
    #[arg(long)]
    bios: Option<String>,

    /// Turn debugging information on
    #[arg(short, long)]
    debug: bool,
}

/// Loads a PlayStation BIOS binary
fn load_bios(path: Option<String>) -> Vec<u8> {
    match path {
        Some(path) => {
            std::fs::read(path.clone()).expect(&format!("Failed to load BIOS file: {}", path))
        }
        None => std::fs::read("bios/bios.bin")
            .expect("Could not load bios/bios.bin. You can specify a different path with --bios"),
    }
}

fn main() {
    // Parse command line arguments
    let args = MainArguments::parse();

    // Create a new emulator instance
    let mut emulator = Emulator::new();

    // Set the debugger to be active if the user requested it
    emulator.debugger.stepping = args.debug;

    // Load the BIOS data from file
    let bios = load_bios(args.bios);
    emulator.cpu.bus.rom.load(bios);

    // Load the executable from the provided path
    if let Some(path) = &args.executable {
        let exe = Executable::load(path).expect("Failed to load executable");

        // Load the executable into the CPU
        exe.load_into(&mut emulator.cpu);
    }

    // Run forever
    emulator.run();
}
