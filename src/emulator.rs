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
    pub joypad_receiver: crossbeam_channel::Receiver<JoypadEvent>,

    in_vblank: bool,
    vsyncs: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JoypadButton {
    Up = 4,
    Down = 6,
    Left = 7,
    Right = 5,
    Start = 3,
    Select = 0,
    Square = 15,
    Triangle = 12,
    Circle = 13,
    Cross = 14,
    L1 = 10,
    R1 = 11,
    L2 = 8,
    R2 = 9,
    L3 = 1,
    R3 = 2,
}

pub enum JoypadAnalog {
    LeftX(i16),
    LeftY(i16),
    RightX(i16),
    RightY(i16),
}

pub enum JoypadEvent {
    Pressed(JoypadButton),
    Released(JoypadButton),
    Analog(JoypadAnalog),
}

impl Emulator {
    /// Create a new emulator instance
    pub fn new() -> (Self, HeapCons<SoundFrame>, crossbeam_channel::Sender<JoypadEvent>) {
        let cpu = Cpu::new();
        let debugger = Debugger::new();
        let breakpoint = debugger.triggered.clone();
        ctrlc::set_handler(move || {
            breakpoint.store(true, Ordering::SeqCst);
        })
        .expect("Error setting Ctrl-C handler");

        let rb = SharedRb::<Heap<SoundFrame>>::new(44100);
        let (sample_producer, sample_consumer) = rb.split();

        let (sender, receiver) = crossbeam_channel::unbounded::<JoypadEvent>();

        (
            Emulator {
                cpu,
                debugger,
                cycles: 0,
                sample_producer,
                joypad_receiver: receiver,
                in_vblank: false,
                vsyncs: 0,
            },
            sample_consumer,
            sender
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

            if false {
                for y in 0..512 {
                    for x in 0..1024 {
                        let (r, g, b) =
                            emulator.cpu.bus.gpu.get_raw_vram_color(x, y);

                        let offset = (y * 1024 + x) * 4;
                        frame_data[offset] = r; // Red
                        frame_data[offset + 1] = g; // Green
                        frame_data[offset + 2] = b; // Blue
                        frame_data[offset + 3] = 255; // Alpha (opaque)
                    }
                }
            } else {
                let (rx, ry) = emulator.cpu.bus.gpu.effective_resolution();
                for y in 0..ry {
                    for x in 0..rx {
                        let (r, g, b) =
                            emulator.cpu.bus.gpu.get_screen_pixel(x, y);

                        let offset = (y * 1024 + x) * 4;
                        frame_data[offset] = r; // Red
                        frame_data[offset + 1] = g; // Green
                        frame_data[offset + 2] = b; // Blue
                        frame_data[offset + 3] = 255; // Alpha (opaque)
                    }
                }
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
        while self.cpu.npc != pc {
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

        if self.cpu.bus.dma.get_and_clear_new_irq() {
            self.cpu.bus.interrupts.trigger_irq(3);
        }

        self.cpu
            .cop0
            .set_hardware_interrupt(self.cpu.bus.interrupts.should_interrupt());

        self.cpu.step();
        self.cycles += 2;

        if self.cpu.bus.dma.pending {
            self.cpu.bus.dma.delay_cycles -= 2;
            if self.cpu.bus.dma.delay_cycles <= 0 {
                println!(
                    "[Emulator] DMA resumed at PC: {:#x}",
                    self.cpu.pc
                );
                self.cpu.bus.handle_dma_write();
            }
        }

        let intc = &mut self.cpu.bus.interrupts;
        self.cpu.bus.gpu.update(self.cycles, intc);
        self.cpu.bus.timers.clock(self.cycles, &self.cpu.bus.gpu, intc);

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

        if self.cycles % 50 == 0 {
            self.cpu.bus.joy.cycle(self.cycles, intc);
        }

        if self.cycles % 768 == 0 {
            // Handle joypad events
            while let Ok(event) = self.joypad_receiver.try_recv() {
                match event {
                    JoypadEvent::Pressed(button) => {
                        self.cpu.bus.joy.press_button(button);
                    }
                    JoypadEvent::Released(button) => {
                        self.cpu.bus.joy.release_button(button);
                    }
                    JoypadEvent::Analog(analog) => {
                        // self.cpu.bus.joy.set_analog(analog);
                    }
                }
            }

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
