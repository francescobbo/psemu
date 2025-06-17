use ringbuf::{HeapRb, traits::*};
use std::sync::atomic::Ordering;

use ringbuf::storage::Heap;
use ringbuf::traits::{Producer, Split};
use ringbuf::{HeapCons, HeapProd, SharedRb};
use winit::event_loop::EventLoopProxy;

use crate::app::{AppEvent, SoundFrame};
use crate::cpu::Cpu;
use crate::debugger::Debugger;

pub struct Emulator {
    pub cpu: Cpu,
    pub debugger: Debugger,

    pub cycles: u64,

    pub sample_producer: HeapProd<SoundFrame>,

    in_vblank: bool,
    vsyncs: u64,
}

impl Emulator {
    /// Create a new emulator instance
    pub fn new() -> (Self, HeapCons<SoundFrame>) {
        let cpu = Cpu::new();
        let debugger = Debugger::new();
        let breakpoint = debugger.triggered.clone();
        ctrlc::set_handler(move || {
            breakpoint.store(true, Ordering::SeqCst);
        })
        .expect("Error setting Ctrl-C handler");

        let rb = SharedRb::<Heap<SoundFrame>>::new(44100);
        let (sample_producer, sample_consumer) = rb.split();

        (
            Emulator {
                cpu,
                debugger,
                cycles: 0,
                sample_producer,
                in_vblank: false,
                vsyncs: 0,
            },
            sample_consumer,
        )
    }

    pub fn run_threaded(
        mut emulator: Emulator,
        event_loop_proxy: EventLoopProxy<AppEvent>,
    ) {
        loop {
            let t0 = std::time::Instant::now();

            // run for approximately 1/60th of a second
            if emulator.run_for_cycles(677_376) {
                // shutdown requested by debugger prompt
                break;
            }

            // Get VRAM frame data (stub: all white)
            let mut frame_data: Vec<u8> = vec![0; 1024 * 512 * 4];
            for (j, pixel) in frame_data.chunks_exact_mut(4).enumerate() {
                let (r, g, b) = emulator.cpu.bus.gpu.get_pixel_color(j);

                pixel[0] = r;
                pixel[1] = g;
                pixel[2] = b;
                pixel[3] = 255; // Alpha channel
            }

            // Send the frame data to the UI thread
            if event_loop_proxy
                .send_event(AppEvent::FrameReady(frame_data))
                .is_err()
            {
                break;
            }

            let t1 = std::time::Instant::now();
            let elapsed = t1.duration_since(t0);
            let target_duration = std::time::Duration::from_millis(16); // ~60 FPS
            // if elapsed < target_duration {
            //     std::thread::sleep(target_duration - elapsed);
            // }
        }

        let _ = event_loop_proxy.send_event(AppEvent::EmulatorShutdown);
    }

    /// Run the emulator
    pub fn run(&mut self) {
        loop {
            if self.step() {
                // Exit if the debugger has requested to quit
                break;
            }
        }
    }

    /// Run the emulator for a specified number of cycles.
    pub fn run_for_cycles(&mut self, cycles: u64) -> bool {
        for _ in 0..cycles {
            if self.step() {
                // Exit if the debugger has requested to quit
                return true;
            }
        }

        false
    }

    pub fn run_until(&mut self, pc: u32) -> bool {
        while self.cpu.pc != pc {
            if self.step() {
                // Exit if the debugger has requested to quit
                return true;
            }
        }
        false
    }

    // Perform one step of the emulator cycle.
    pub fn step(&mut self) -> bool {
        let break_reason = self.debugger.break_reason(&self.cpu);
        if let Some(reason) = break_reason {
            if self.debugger.enter(&mut self.cpu, reason) {
                // If the debugger returns true, it means we should quit
                println!("Quitting...");
                return true;
            }
        }

        // Detect the putchar system call and print the character to the
        // console
        if self.cpu.pc == 0xb0 && self.cpu.registers[9] == 0x3d {
            print!("{}", self.cpu.registers[4] as u8 as char);
        }

        self.cpu
            .cop0
            .set_hardware_interrupt(self.cpu.bus.interrupts.should_interrupt());

        self.cpu.step();
        self.cycles += 2;

        let intc = &mut self.cpu.bus.interrupts;
        self.cpu.bus.gpu.update(self.cycles, intc);
        self.cpu.bus.timers.clock(self.cycles, &self.cpu.bus.gpu);

        if self.in_vblank != self.cpu.bus.gpu.is_in_vblank {
            self.in_vblank = self.cpu.bus.gpu.is_in_vblank;
            if self.in_vblank {
                self.vsyncs += 1;
            }
        }

        if self.cycles % 33868800 == 0 {
            println!(
                "[Emulator] VSync: {}, Cycles: {}, PC: {:#x}",
                self.vsyncs, self.cycles, self.cpu.pc
            );
        }

        if self.cycles % 769 == 0 {
            self.cpu.bus.cdrom.clock(intc);

            // Produce a sound frame every 768 cycles (approximately 44100 Hz)
            let sample = 0.0;
            self.cpu.bus.spu.tick();
            let sf = crate::app::SoundFrame(sample, sample);

            // while self.sample_producer.is_full() {
            //     // wait for the consumer to catch up
            // }

            if self.sample_producer.try_push(sf).is_err() {
                // eprintln!("Sound buffer is full, dropping samples");
            }
        }

        false
    }
}
