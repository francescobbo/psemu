mod app;
mod bus;
mod cpu;
mod debug;
mod emulator;
mod executable;
mod interrupts;
mod ram;
mod rom;

use clap::Parser;
use winit::event_loop::EventLoop;

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

fn main() {
    let event_loop = EventLoop::with_user_event()
        .build()
        .expect("Failed to create event loop");

    let mut app = app::App::new();
    event_loop
        .run_app(&mut app)
        .expect("Failed to run application");
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
