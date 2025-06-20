use crate::gpu::Gpu;

#[derive(Debug, Default)]
pub struct Timers {
    timers: [Timer; 3],
    last_cpu_cycles: u64,
    dotclock_counter: f64,
    t2_cpu_cycles_buffer: u32,

    in_hblank: bool,
    in_vblank: bool,
}

#[derive(Debug, Default)]
struct Timer {
    counter: u16,
    target: u16,

    is_synchronized: bool,
    sync_mode: u8,
    reset_at_target: bool,
    irq_at_target: bool,
    irq_at_overflow: bool,
    irq_repeat_mode: bool,
    irq_pulse_mode: bool,
    clock_source: u8,
    irq_neg: bool,
    reached_target: bool,
    reached_overflow: bool,
}

impl Timers {
    /// Creates a new Timers instance with all counters initialized to zero.
    pub fn new() -> Self {
        Timers::default()
    }

    pub fn clock(&mut self, cpu_cycles: u64, gpu: &Gpu) {
        let started_hblank = gpu.is_in_hblank && !self.in_hblank;
        let started_vblank = gpu.is_in_vblank && !self.in_vblank;

        // Update the hblank and vblank states
        self.in_hblank = gpu.is_in_hblank;
        self.in_vblank = gpu.is_in_vblank;

        let cycles_diff = cpu_cycles - self.last_cpu_cycles;
        self.last_cpu_cycles = cpu_cycles;

        self.run_t0(
            cycles_diff,
            gpu.cpu_clocks_to_dotclocks(cycles_diff),
            started_hblank,
        );

        self.run_t1(cycles_diff, started_hblank, started_vblank);

        self.run_t2(cycles_diff);
    }

    pub fn run_t0(
        &mut self,
        cpu_cycles: u64,
        dotclock_cycles: f64,
        started_hblank: bool,
    ) {
        let timer = &mut self.timers[0];
        if timer.is_synchronized {
            match timer.sync_mode {
                // If sync mode is 0, t0 is paused during hblank
                0 => {
                    if self.in_hblank {
                        return; // Pause during hblank
                    }
                }
                // if sync mode is 1, t0 is reset at the start of hblank
                1 => {
                    if started_hblank {
                        timer.counter = 0;
                    }
                }
                // if sync mode is 2, t0 is reset at the start of hblank, and only runs during hblank
                2 => {
                    if !self.in_hblank {
                        return;
                    }

                    if started_hblank {
                        timer.counter = 0;
                    }
                }
                // if sync mode is 3, t0 is halted until the next hblank, then synchronization is disabled
                3 => {
                    if started_hblank {
                        timer.is_synchronized = false;
                    } else {
                        return; // Halt until the next hblank
                    }
                }
                _ => {}
            }
        }

        let (of, target) = if timer.clock_source == 0 || timer.clock_source == 2
        {
            // when clock source is 0 or 2, t0 follows the CPU clock
            timer.add_counter(cpu_cycles)
        } else {
            // otherwise, t0 follows the GPU dotclock
            self.dotclock_counter += dotclock_cycles;

            // take the integral of the dotclock cycles
            let integral_cycles = self.dotclock_counter as u64;
            self.dotclock_counter -= integral_cycles as f64;
            timer.add_counter(integral_cycles)
        };

        if of {
            // If the overflow flag is set, we need to handle it
            timer.reached_overflow = true;
            if timer.irq_at_overflow {
                timer.irq_neg = false; // Set IRQ pending
            }
        }
        if target {
            // If the target flag is set, we need to handle it
            timer.reached_target = true;
            if timer.irq_at_target {
                timer.irq_neg = false; // Set IRQ pending
            }
        }
    }

    pub fn run_t1(
        &mut self,
        cpu_cycles: u64,
        started_hblank: bool,
        started_vblank: bool,
    ) {
        let timer = &mut self.timers[1];
        if timer.is_synchronized {
            match timer.sync_mode {
                // If sync mode is 0, t1 is paused during vblank
                0 => {
                    if self.in_vblank {
                        return; // Pause during vblank
                    }
                }
                // if sync mode is 1, t1 is reset at the start of vblank
                1 => {
                    if started_vblank {
                        timer.counter = 0;
                    }
                }
                // if sync mode is 2, t1 is reset at the start of vblank, and only runs during vblank
                2 => {
                    if !self.in_vblank {
                        return;
                    }

                    if started_vblank {
                        timer.counter = 0;
                    }
                }
                // if sync mode is 3, t1 is halted until the next hblank, then synchronization is disabled
                3 => {
                    if started_vblank {
                        timer.is_synchronized = false;
                    } else {
                        return; // Halt until the next hblank
                    }
                }
                _ => {}
            }
        }

        let (of, target) = if timer.clock_source == 0 || timer.clock_source == 2
        {
            // when clock source is 0 or 2, t1 follows the CPU clock
            timer.add_counter(cpu_cycles)
        } else {
            // otherwise, t1 is incremented at each hblank
            if started_hblank {
                timer.add_counter(1)
            } else {
                return; // Do not increment during hblank
            }
        };

        if of {
            // If the overflow flag is set, we need to handle it
            timer.reached_overflow = true;
            if timer.irq_at_overflow {
                timer.irq_neg = false; // Set IRQ pending
            }
        }
        if target {
            // If the target flag is set, we need to handle it
            timer.reached_target = true;
            if timer.irq_at_target {
                timer.irq_neg = false; // Set IRQ pending
            }
        }
    }

    pub fn run_t2(&mut self, cpu_cycles: u64) {
        let timer = &mut self.timers[2];
        if timer.is_synchronized {
            match timer.sync_mode {
                // If sync mode is 0 or 3, t2 is paused.
                0 | 3 => return,
                _ => {}
            }
        }

        let (of, target) = if timer.clock_source == 0 || timer.clock_source == 1
        {
            // when clock source is 0 or 1, t2 follows the CPU clock
            timer.add_counter(cpu_cycles)
        } else {
            // otherwise, t2 follows the CPU clock divided by 8
            self.t2_cpu_cycles_buffer += cpu_cycles as u32;

            let mut t2_cycles = 0;
            while self.t2_cpu_cycles_buffer >= 8 {
                // when clock source is 1, t0 follows the GPU dotclock
                // we need to convert the CPU cycles to dotclock cycles
                self.t2_cpu_cycles_buffer -= 8;
                t2_cycles += 1;
            }

            timer.add_counter(t2_cycles as u64)
        };

        if of {
            // If the overflow flag is set, we need to handle it
            timer.reached_overflow = true;
            if timer.irq_at_overflow {
                timer.irq_neg = false; // Set IRQ pending
            }
        }
        if target {
            // If the target flag is set, we need to handle it
            timer.reached_target = true;
            if timer.irq_at_target {
                timer.irq_neg = false; // Set IRQ pending
            }
        }
    }

    /// Reads the value of the specified timer.
    pub fn read(&mut self, address: u32) -> u32 {
        let address = address - 0x1f801100;
        let timer_idx = address >> 4;

        let timer = &mut self.timers[timer_idx as usize];
        match address & 0x0f {
            0x00 => timer.counter as u32,
            0x04 => Self::read_control(timer),
            0x08 => timer.target as u32,
            _ => panic!("Invalid timer address: {:#x}", address),
        }
    }

    /// Writes a value to the specified timer.
    pub fn write(&mut self, address: u32, value: u32) {
        let address = address - 0x1f801100;
        let timer_idx = address >> 4;
        let value = value & 0xffff;

        let timer = &mut self.timers[timer_idx as usize];
        match address & 0x0f {
            0x00 => timer.counter = value as u16,
            0x04 => Self::write_control(timer, value),
            0x08 => {
                println!(
                    "[Timers] Setting timer {} target to {:#x}",
                    timer_idx, value
                );
                timer.target = value as u16
            }
            _ => panic!("Invalid timer address: {:#x}", address),
        }
    }

    /// Reads the control register of the timer.
    fn read_control(timer: &mut Timer) -> u32 {
        let mut control = timer.is_synchronized as u32;
        control |= (timer.sync_mode as u32) << 1;
        control |= (timer.reset_at_target as u32) << 3;
        control |= (timer.irq_at_target as u32) << 4;
        control |= (timer.irq_at_overflow as u32) << 5;
        control |= (timer.irq_repeat_mode as u32) << 6;
        control |= (timer.irq_pulse_mode as u32) << 7;
        control |= (timer.clock_source as u32) << 8;
        control |= (timer.irq_neg as u32) << 10;
        control |= (timer.reached_target as u32) << 11;
        control |= (timer.reached_overflow as u32) << 12;

        // Reset the reached flags
        timer.reached_target = false;
        timer.reached_overflow = false;

        control
    }

    /// Writes the control register of the timer.
    fn write_control(timer: &mut Timer, value: u32) {
        // Writes to the control register acknowledge any pending IRQ
        timer.irq_neg = true; // true means "no IRQ pending", hence "neg"

        // Writes to the control register reset the counter
        timer.counter = 0;

        timer.is_synchronized = (value & 0x01) != 0;
        timer.sync_mode = ((value >> 1) & 0x03) as u8;
        timer.reset_at_target = (value & 0x08) != 0;
        timer.irq_at_target = (value & 0x10) != 0;
        timer.irq_at_overflow = (value & 0x20) != 0;
        timer.irq_repeat_mode = (value & 0x40) != 0;
        timer.irq_pulse_mode = (value & 0x80) != 0;
        timer.clock_source = ((value >> 8) & 0x03) as u8;
        timer.irq_neg = (value & 0x400) != 0;

        // Reset the reached flags
        timer.reached_target = false;
        timer.reached_overflow = false;
    }
}

impl Timer {
    /// Adds the specified number of CPU cycles to the timer's counter.
    /// Returns a tuple indicating whether the overflow or target was reached.
    fn add_counter(&mut self, mut steps: u64) -> (bool, bool) {
        let mut of = false;
        let mut target = false;
        // Check if adding the steps would cross the target.
        // This is made more complicated by the fact that the counter is a u16,
        // so we need to handle the wrap-around case.
        let cap = if self.reset_at_target {
            if self.target > 0 {
                self.target as u16
            } else {
                0xffff // If target is 0, we use the maximum value of the counter
            }
        } else {
            0xffff
        };

        if self.counter > cap {
            // edge case, can happen if manually setting the counter to a value greater than the target
            panic!("DO SOMETHING")
        }

        let distance = cap - self.counter;
        if steps >= distance as u64 {
            // If the steps are greater than or equal to the distance to the target,
            // we reach the target and overflow.
            self.counter = 0;
            steps -= distance as u64;
            self.counter += steps as u16;

            if self.reset_at_target {
                target = true;
            } else {
                of = true;
            }
        } else {
            // If the steps are less than the distance to the target,
            // we just increment the counter.
            self.counter += steps as u16;
        }

        (of, target)
    }
}
