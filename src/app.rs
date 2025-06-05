use std::sync::Arc;

use pixels::{Pixels, SurfaceTexture};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

use crate::{MainArguments, emulator::Emulator, executable::Executable};

pub struct App {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
}

impl App {
    pub fn new(args: MainArguments) -> Self {
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

        std::thread::spawn(move || {
            Emulator::run_threaded(emulator);
        });

        Self {
            window: None,
            pixels: None,
        }
    }

    pub fn render(&mut self) {
        if self.pixels.is_none() {
            // No pixels instance available, we are still initializing
            return;
        }

        // Take pixels out of the Option
        let pixels = self.pixels.as_mut().unwrap();

        // Get the pixel buffer as a mutable array.
        let frame = pixels.frame_mut();

        // Fill the frame with a color (e.g., blue: 00,00,FF,FF in RGBA)
        for pixel_chunk in frame.chunks_exact_mut(4) {
            pixel_chunk[0] = 0x00; // R
            pixel_chunk[1] = 0x00; // G
            pixel_chunk[2] = 0xff; // B
            pixel_chunk[3] = 0xff; // A (opaque)
        }

        // Now that we have filled the pixel buffer, we ask pixels to present it
        // to the window.
        pixels.render().unwrap();
    }
}

const INITIAL_WIDTH: u32 = 1024;
const INITIAL_HEIGHT: u32 = 512;

impl ApplicationHandler for App {
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
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, window);

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
            WindowEvent::RedrawRequested => self.render(),
            WindowEvent::Resized(size) => {
                if let Some(pixels) = &mut self.pixels {
                    pixels.resize_surface(size.width, size.height).unwrap();
                }
            }
            _ => {}
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
