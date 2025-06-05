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

    let mut app = app::App::new(MainArguments::parse());
    event_loop
        .run_app(&mut app)
        .expect("Failed to run application");
}
