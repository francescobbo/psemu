use cpal::{
    StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use std::sync::Arc;

use pixels::{Pixels, SurfaceTexture};
use ringbuf::{
    HeapCons,
    traits::{Consumer, Observer},
};
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

    output_device: Option<cpal::Device>,
    sound_stream: Option<cpal::Stream>,
}

#[derive(Debug, Clone, Copy)]
pub struct SoundFrame(pub f32, pub f32);

impl App {
    pub fn new(
        args: MainArguments,
        event_loop_proxy: EventLoopProxy<AppEvent>,
    ) -> Self {
        // Create a new emulator instance
        let (mut emulator, sample_consumer) = Emulator::new();

        // Set the debugger to be active if the user requested it
        emulator.debugger.steps = args.debug as usize;

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

        let host = cpal::default_host();
        let output_device = host.default_output_device();

        let mut s = Self {
            window: None,
            pixels: None,
            output_device: output_device,
            sound_stream: None,
        };

        s.start_audio(sample_consumer);
        s
    }

    fn start_audio(&mut self, mut sample_consumer: HeapCons<SoundFrame>) {
        if self.output_device.is_none() {
            eprintln!("No output device available for audio playback.");
            return;
        }

        let device = self.output_device.as_ref().unwrap();

        let supported_config = device.default_output_config().unwrap();
        let sample_format = supported_config.sample_format();
        if sample_format != cpal::SampleFormat::F32 {
            eprintln!("Unsupported sample format: {:?}", sample_format);
            return;
        }

        let config: StreamConfig = supported_config.into();
        let sample_rate = config.sample_rate.0 as f32;
        if sample_rate != 44100.0 {
            eprintln!(
                "Unsupported sample rate: {}. Expected 44100Hz",
                sample_rate
            );
            return;
        }

        let channels = config.channels as usize;

        let mut last_sample: SoundFrame = SoundFrame(0.0, 0.0);
        let stream = device
            .build_output_stream(
                &config,
                move |output: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    // Fill output buffer with sine wave
                    for frame in output.chunks_mut(channels) {
                        if let Some(sound_frame) = sample_consumer.try_pop() {
                            // Use the sound frame's value
                            frame[0] = sound_frame.0 * 5.0;
                            frame[1] = sound_frame.1 * 5.0;
                            last_sample = sound_frame;

                            // println!(
                            //     "Popped sound frame: ({}, {})",
                            //     sound_frame.0, sound_frame.1
                            // );
                        } else {
                            // If no sound frame is available, fill with silence
                            frame[0] = last_sample.0;
                            frame[1] = last_sample.1;
                        }
                    }

                    // Reduce the data in the ring buffer to the size of the output buffer,
                    // for next time this callback is called. That data is never getting
                    // played.
                    let cap = output.len() / channels;
                    if sample_consumer.occupied_len() > cap {
                        // Skip the excess samples in the ring buffer
                        println!(
                            "Skipping {} samples in the ring buffer",
                            sample_consumer.occupied_len() - cap
                        );
                        sample_consumer
                            .skip(sample_consumer.occupied_len() - cap);
                        println!(
                            "New ring buffer size: {}",
                            sample_consumer.occupied_len()
                        );
                    }
                },
                |err| eprintln!("Audio error: {}", err),
                None,
            )
            .unwrap();

        // Start the audio stream
        if let Err(e) = stream.play() {
            eprintln!("Failed to start audio stream: {}", e);
        } else {
            println!("Audio stream started successfully.");
        }

        self.sound_stream = Some(stream);
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
