use pixels::wgpu::Color;

use crate::bus::AccessSize;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Direction {
    ToRam,
    FromRam,
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum Step {
    Forward,
    Backward,
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum Chopping {
    Disabled,
    Enabled,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SyncMode {
    Immediate,
    Sync,
    LinkedList,
    Reserved,
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum Busy {
    Available,
    Busy,
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum Trigger {
    Stop,
    Start,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ChannelLink {
    MdecIn = 0,
    MdecOut = 1,
    Gpu = 2,
    Cdrom = 3,
    Spu = 4,
    Pio = 5,
    Otc = 6,
}

impl ChannelLink {
    pub fn get(i: u32) -> ChannelLink {
        match i {
            0 => ChannelLink::MdecIn,
            1 => ChannelLink::MdecOut,
            2 => ChannelLink::Gpu,
            3 => ChannelLink::Cdrom,
            4 => ChannelLink::Spu,
            5 => ChannelLink::Pio,
            6 => ChannelLink::Otc,
            _ => unreachable!(),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Channel {
    n: u32,
    link: ChannelLink,

    base: u32,
    channel_control: u32,

    block_size: u32,
    block_count: u32,

    direction: Direction,
    step: Step,
    chopping: Chopping,
    sync_mode: SyncMode,
    chopping_dma_window: u32,
    chopping_cpu_window: u32,
    busy: Busy,
    trigger: Trigger,
}

pub struct Dma {
    dpcr: u32,
    dicr: u32,
    channels: [Channel; 7],
    pub delay_cycles: i32,
    pub pending: bool,

    new_irq: bool,
}

impl Dma {
    pub fn new() -> Dma {
        Dma {
            dpcr: 0x0765_4321,
            dicr: 0,
            channels: [
                Channel::new(0),
                Channel::new(1),
                Channel::new(2),
                Channel::new(3),
                Channel::new(4),
                Channel::new(5),
                Channel::new(6),
            ],
            delay_cycles: 0,
            pending: false,

            new_irq: false,
        }
    }

    pub fn read(&mut self, addr: u32, size: AccessSize) -> u32 {
        if size != AccessSize::Word {
            println!("Unhandled {size:?} DMA read");
            return 0;
        }

        match addr {
            0x00..=0x6f => {
                let channel = (addr >> 4) as usize;
                self.channels[channel].read(addr & 0xf, size)
            }
            0x70 => self.dpcr,
            0x74 => self.dicr,
            0x78 => unimplemented!(),
            0x7c => unimplemented!(),
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, addr: u32, value: u32, size: AccessSize) {
        if size != AccessSize::Word {
            panic!("Unhandled {size:?} DMA write");
        }

        match addr {
            0x00..=0x6f => {
                let channel = (addr >> 4) as usize;
                self.channels[channel].write(addr & 0xf, value, size);
            }
            0x70 => {
                self.dpcr = value;
            }
            0x74 => {
                self.write_dicr(value);
                // println!("[DMA] Wrote {:08x} to DICR, resulting in new DICR:
                // {:08x}", value, self.dicr);
            }
            0x78 => unimplemented!(),
            0x7c => unimplemented!(),
            _ => unreachable!(),
        };
    }

    pub fn irq(&mut self, channel: usize) {
        if self.dicr & (1 << (16 + channel)) != 0 {
            // If the IRQ is enabled, set the corresponding bit in DICR
            self.dicr |= 1 << (24 + channel);
        }

        self.update_irq_flag();
    }

    pub fn update_irq_flag(&mut self) {
        // Compute bit31 (IRQ active)
        let mask = self.dicr >> 16 & 0x7f;
        let active = self.dicr >> 24 & 0x7f;
        let error_flag = self.dicr & (1 << 15) != 0;
        let master_enable = self.dicr & (1 << 23) != 0;

        let irq_active =
            if error_flag || (master_enable && (active & mask) != 0) {
                1 << 31
            } else {
                0
            };

        if !self.new_irq && irq_active != 0 {
            // If the IRQ was not already active, set the new_irq flag
            self.new_irq = true;
        }

        self.dicr = (self.dicr & !0x8000_0000) | irq_active;
    }

    pub fn get_and_clear_new_irq(&mut self) -> bool {
        let new_irq = self.new_irq;
        self.new_irq = false;
        new_irq
    }

    pub fn active_channel(&mut self) -> Option<&mut Channel> {
        let channels_by_priority = self.channels_by_priority();
        
        for pi in 0..channels_by_priority.len() {
            let i = channels_by_priority[pi];
            let link = self.channels[i].link(); // immutable borrow for enabled()
            if self.enabled(link) {
                // Now we can mutably borrow
                if self.channels[i].active() {
                    return Some(&mut self.channels[i]);
                }
            }
        }

        None
    }

    pub fn channels_by_priority(&self) -> [usize; 7] {
        // The DPCR register defines a priority score for each channel, encoded like:
        // 0-2   DMA0, MDECin  Priority      (0..7; 0=Highest, 7=Lowest)
        // 4-6   DMA1, MDECout Priority      (0..7; 0=Highest, 7=Lowest)
        // 8-10  DMA2, GPU     Priority      (0..7; 0=Highest, 7=Lowest)
        // etc.
        // if two channels have the same priority score, then they are sorted
        // by their number (channel 6 has higher priority than channel 5).

        let mut scored: [(usize, usize); 7] = [0, 1, 2, 3, 4, 5, 6].map(|ch| {
            let score = ((self.dpcr >> (ch * 4)) & 3) as usize;
            (ch, score)
        });

        // Sort by score ascending, then by channel descending
        scored.sort_by(|&(ch_a, score_a), &(ch_b, score_b)| {
            score_a
                .cmp(&score_b)
                .then_with(|| ch_b.cmp(&ch_a))
        });

        // Strip off the scores, returning just the channel numbers
        scored.map(|(ch, _)| ch)
    }

    fn enabled(&self, link: ChannelLink) -> bool {
        // Check if the channel is enabled in DPCR
        let bit = match link {
            ChannelLink::MdecIn => 3,
            ChannelLink::MdecOut => 7,
            ChannelLink::Gpu => 11,
            ChannelLink::Cdrom => 15,
            ChannelLink::Spu => 19,
            ChannelLink::Pio => 23,
            ChannelLink::Otc => 27,
        };

        (self.dpcr & (1 << bit)) != 0
    }

    fn write_dicr(&mut self, value: u32) {
        // Clear fixed-zero bits
        let value = value & !0x7fc0;

        let rw_parts = value & 0xff_ffff;
        let acks = value & 0x7f00_0000;

        // println!("[DMA] Writing to DICR: {:08x} (rw_parts: {:08x}, acks: {:08x})", value, rw_parts, acks);

        // Replace the low 0-23 bits with the new value
        self.dicr &= !0xff_ffff;
        self.dicr |= rw_parts;

        // Ack IRQs in bits 24-30
        let currently_active_irqs = self.dicr & 0x7f00_0000;
        let new_active_irqs = currently_active_irqs & !acks;

        // Replace the IRQ flags
        self.dicr &= !0x7f00_0000;
        self.dicr |= new_active_irqs << 24;

        // Compute bit31 (IRQ active)
        let force_irq = self.dicr & (1 << 15) != 0;
        let master_enable = self.dicr & (1 << 23) != 0;
        let enabled_irqs = (self.dicr >> 16) & 0x7f;
        let active_irqs = (self.dicr >> 24) & 0x7f;

        let irq_active = if force_irq
            || (master_enable && (active_irqs & enabled_irqs) != 0)
        {
            1 << 31
        } else {
            0
        };

        self.dicr &= !(1 << 31);
        self.dicr |= irq_active;

        if !self.new_irq && irq_active != 0 {
            self.new_irq = true;
        }
    }
}

impl Channel {
    pub fn new(n: u32) -> Channel {
        Channel {
            n,
            link: ChannelLink::get(n),
            base: 0,
            channel_control: if n == 6 { 1 } else { 0 },

            block_size: 0,
            block_count: 0,

            direction: Direction::ToRam,
            step: if n == 6 {
                Step::Backward
            } else {
                Step::Forward
            },
            chopping: Chopping::Disabled,
            sync_mode: SyncMode::Immediate,
            chopping_dma_window: 0,
            chopping_cpu_window: 0,
            busy: Busy::Available,
            trigger: Trigger::Stop,
        }
    }

    fn read(&mut self, addr: u32, size: AccessSize) -> u32 {
        match addr {
            0x00 => self.base,
            0x04 => self.read_block_control(),
            0x08 => {
                // println!("[DMA] READ D{}_CHCR = {:08x};  State: {:?}; Trigger: {:?}; Sync:
                // {:?}; Dir: {:?}; Step: {:?}; Chop: {:?}", self.n, self.
                // channel_control, self.busy, self.trigger, self.sync_mode, self.direction,
                // self.step, self.chopping);

                self.channel_control
            }
            _ => {
                unreachable!()
            }
        }
    }

    fn write(&mut self, addr: u32, value: u32, size: AccessSize) {
        // println!("[DMA] write {:08x} to {:08x}", value, addr);
        match addr {
            0x00 => self.set_base(value),
            0x04 => self.set_block_control(value),
            0x08 => self.set_channel_control(value),
            _ => unreachable!(),
        };
    }

    pub fn active(&self) -> bool {
        if self.sync_mode == SyncMode::Immediate {
            self.busy == Busy::Busy && self.trigger == Trigger::Start
        } else {
            self.busy == Busy::Busy
        }
    }

    pub fn link(&self) -> ChannelLink {
        self.link
    }

    pub fn step(&self) -> i32 {
        match self.step {
            Step::Backward => -4,
            Step::Forward => 4,
        }
    }

    pub fn base(&self) -> u32 {
        self.base
    }

    pub fn transfer_size(&self) -> (u32, u32) {
        if self.block_size == 0 {
            return (self.block_count, 0x10000);
        }

        (self.block_count, self.block_size)
    }

    pub fn sync_mode(&self) -> SyncMode {
        self.sync_mode
    }

    pub fn chopping(&self) -> bool {
        self.chopping == Chopping::Enabled
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }

    pub fn done(&mut self) {
        self.busy = Busy::Available;
        self.trigger = Trigger::Stop;

        self.channel_control &= !((1 << 24) | (1 << 28));
    }

    fn read_block_control(&self) -> u32 {
        (self.block_count << 16) | self.block_size
    }

    pub fn set_base(&mut self, value: u32) {
        self.base = value & 0xff_ffff;

        // println!("[DMA] D{}_MADR = {:08x}", self.n, self.base);
    }

    fn set_block_control(&mut self, value: u32) {
        self.block_size = value & 0xffff;
        self.block_count = value >> 16;

        // println!("[DMA] D{}_BCR = {} x {} words", self.n, self.block_count,
        // self.block_size)
    }

    fn set_channel_control(&mut self, mut value: u32) {
        // Cleanup zero bits
        if self.n == 6 {
            // On DMA6 only b24, b28 and b30 are R/W
            // b1 is always 1 and the rest is 0.
            value = (value & 0x5100_0000) | (1 << 1);
        } else {
            value &= !0x8e88_f8fc;
        }

        self.direction = match value & 1 != 0 {
            false => Direction::ToRam,
            true => Direction::FromRam,
        };

        self.step = match value & (1 << 1) != 0 {
            false => Step::Forward,
            true => Step::Backward,
        };

        self.chopping = match value & (1 << 8) != 0 {
            false => Chopping::Disabled,
            true => Chopping::Enabled,
        };

        self.sync_mode = match (value >> 9) & 3 {
            0 => SyncMode::Immediate,
            1 => SyncMode::Sync,
            2 => SyncMode::LinkedList,
            4 => SyncMode::Reserved,
            _ => unreachable!(),
        };

        self.chopping_dma_window = (value >> 16) & 7;
        self.chopping_cpu_window = (value >> 20) & 7;

        self.busy = match value & (1 << 24) != 0 {
            false => Busy::Available,
            true => Busy::Busy,
        };

        self.trigger = match value & (1 << 28) != 0 {
            false => Trigger::Stop,
            true => Trigger::Start,
        };

        self.channel_control = value;

        // println!("[DMA] D{}_CHCR = {:08x};  State: {:?}; Trigger: {:?}; Sync:
        // {:?}; Dir: {:?}; Step: {:?}; Chop: {:?}", self.n, self.
        // channel_control, self.busy, self.trigger, self.sync_mode,
        // self.direction, self.step, self.chopping);
    }
}
