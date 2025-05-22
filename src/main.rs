mod bus;
mod cpu;
mod debug;
mod emulator;
mod executable;
mod ram;

use bus::AccessSize;
use clap::Parser;
use emulator::Emulator;
use executable::Executable;

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

    // Load the executable from the provided path
    let exe = Executable::load(&args.executable.unwrap()).expect("Failed to load executable");

    // Load the executable into the CPU
    exe.load_into(&mut emulator.cpu);

    // Run forever
    emulator.run();
}
