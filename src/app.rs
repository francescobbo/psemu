use std::sync::Arc;

use pixels::{Pixels, SurfaceTexture};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoopProxy},
    window::{Window, WindowId},
};

use crate::{MainArguments, emulator::Emulator, executable::Executable};

#[derive(Debug)]
pub enum AppEvent {
    /// Event sent when a new frame is ready to be rendered.
    FrameReady(Vec<u8>),

    /// Event sent when the emulator has requested to shut down.
    EmulatorShutdown,
}

pub struct App {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
}

impl App {
    pub fn new(
        args: MainArguments,
        event_loop_proxy: EventLoopProxy<AppEvent>,
    ) -> Self {
        // Create a new emulator instance
        let mut emulator = Emulator::new();

        // Set the debugger to be active if the user requested it
        emulator.debugger.stepping = args.debug;

        // Load the BIOS data from file
        let bios = load_bios(args.bios);
        emulator.cpu.bus.rom.load(bios);

        // Load the executable from the provided path
        if let Some(path) = &args.executable {
            emulator.run_until(0x80030000); // Run until the BIOS entry point

            let exe =
                Executable::load(path).expect("Failed to load executable");

            // Load the executable into the CPU
            exe.load_into(&mut emulator.cpu);
        }

        std::thread::spawn(move || {
            Emulator::run_threaded(emulator, event_loop_proxy);
        });

        Self {
            window: None,
            pixels: None,
        }
    }
}

const INITIAL_WIDTH: u32 = 1024;
const INITIAL_HEIGHT: u32 = 512;

impl ApplicationHandler<AppEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title("PlayStation Emulator")
            .with_min_inner_size(LogicalSize::new(320, 240))
            .with_inner_size(LogicalSize::new(INITIAL_WIDTH, INITIAL_HEIGHT));

        let window = Arc::new(
            event_loop
                .create_window(window_attributes)
                .expect("Failed to create window"),
        );

        self.window = Some(window.clone());
        window.request_redraw();

        let window_size = window.inner_size();
        let surface_texture =
            SurfaceTexture::new(window_size.width, window_size.height, window);

        self.pixels = Some(
            Pixels::new(INITIAL_WIDTH, INITIAL_HEIGHT, surface_texture)
                .expect("Failed to create Pixels instance"),
        );
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                if let Some(pixels) = &self.pixels {
                    // Render the pixels to the window
                    if pixels.render().is_err() {
                        eprintln!("Failed to render pixels");
                    }
                }
            }
            WindowEvent::Resized(size) => {
                if let Some(pixels) = &mut self.pixels {
                    pixels.resize_surface(size.width, size.height).unwrap();
                }
            }
            _ => {}
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::FrameReady(frame_data) => {
                if let Some(pixels) = &mut self.pixels {
                    // Get the pixel buffer as a mutable array.
                    let frame = pixels.frame_mut();

                    // Copy the frame data into the pixel buffer.
                    frame.copy_from_slice(&frame_data);
                }

                // Request a redraw of the window
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            AppEvent::EmulatorShutdown => {
                event_loop.exit();
            }
        }
    }
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
