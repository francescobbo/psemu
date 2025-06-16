mod app;
mod bus;
mod cpu;
mod debugger;
mod dma;
mod emulator;
mod executable;
mod gpu;
mod interrupts;
mod joy;
mod ram;
mod rom;
mod scratchpad;
mod spu;

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
    let event_loop_proxy = event_loop.create_proxy();

    let mut app = app::App::new(MainArguments::parse(), event_loop_proxy);
    event_loop
        .run_app(&mut app)
        .expect("Failed to run application");
}
