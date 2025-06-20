use crate::{
    cdrom::Cdrom,
    dma::{ChannelLink, Direction, Dma, SyncMode},
    gpu::Gpu,
    interrupts::{INTERRUPTS_BASE, INTERRUPTS_END, InterruptController},
    ram::{self, Ram},
    rom::{self, Rom},
    scratchpad::Scratchpad,
    spu::Spu,
    timers::Timers,
};

/// Represents the possible access sizes for memory operations.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AccessSize {
    Byte = 1,
    HalfWord = 2,
    Word = 4,
}

impl TryFrom<usize> for AccessSize {
    type Error = &'static str;

    fn try_from(size: usize) -> Result<Self, Self::Error> {
        match size {
            1 => Ok(AccessSize::Byte),
            2 => Ok(AccessSize::HalfWord),
            4 => Ok(AccessSize::Word),
            _ => Err("Invalid access size"),
        }
    }
}

/// Represents the bus that connects the CPU to the rest of the system.
/// Due to Rust's ownership model, the bus will own the RAM and all the other
/// devices.
pub struct Bus {
    pub ram: Ram,
    pub rom: Rom,
    pub gpu: Gpu,
    pub spu: Spu,
    pub cdrom: Cdrom,
    pub dma: Dma,
    pub timers: Timers,

    pub interrupts: InterruptController,

    /// The BIU control registers.
    /// - 0: Exp. 1 address.
    /// - 1: Exp. 2 address.
    /// - 2: Exp. 1 size and timings.
    /// - 3: Exp. 3 size and timings.
    /// - 4: ROM size and timings.
    /// - 5: SPU size and timings.
    /// - 6: CDROM size and timings.
    /// - 7: Exp. 2 size and timings.
    /// - 8: Common timings.
    biu_control: [u32; 9],

    /// The DRAM control register
    dram_control: u32,

    /// The scratchpad RAM, which is 1KB in size.
    pub scratchpad: Scratchpad,

    pub joy: crate::joy::Joy,
}

const BIU_CONTROL_BASE: u32 = 0x1f80_1000;
const BIU_CONTROL_SIZE: u32 = 9 * 4; // 9 registers, each 4 bytes
const BIU_CONTROL_END: u32 = BIU_CONTROL_BASE + BIU_CONTROL_SIZE - 1;

const DRAM_CONTROL_BASE: u32 = 0x1f80_1060;
const DRAM_CONTROL_SIZE: u32 = 4; // 4 bytes for the DRAM control register
const DRAM_CONTROL_END: u32 = DRAM_CONTROL_BASE + DRAM_CONTROL_SIZE - 1;

const IO_STUBS: &[(u32, u32, &str)] = &[
    (0x1f000000, 0x1f7fffff, "Exp1"),
    (0x1f801050, 0x1f80105f, "Serial"),
    (0x1f801820, 0x1f801827, "MDEC"),
    (0x1f802000, 0x1f803fff, "Exp2"),
    (0x1fa00000, 0x1fbfffff, "Exp3"),
];

impl Bus {
    /// Creates a new system bus.
    pub fn new() -> Self {
        Self {
            ram: Ram::new(),
            rom: Rom::new(),
            gpu: Gpu::new(),
            spu: Spu::new(),
            cdrom: Cdrom::new(),
            dma: Dma::new(),
            timers: Timers::new(),
            interrupts: InterruptController::new(),
            biu_control: [0; 9],
            dram_control: 0,
            scratchpad: Scratchpad::new(),

            joy: crate::joy::Joy::new(),
        }
    }

    /// Performs a read operation on the bus.
    pub fn read(&mut self, address: u32, size: AccessSize) -> Result<u32, ()> {
        for &(start, end, name) in IO_STUBS {
            if address >= start && address <= end {
                println!("[{name}] Reading ({size:?}) at {address:08x}");
                return Ok(0);
            }
        }

        match address {
            0x1f80_0000..=0x1f80_03ff => {
                // Scratchpad RAM
                Ok(self.scratchpad.read(address, size))
            }
            ram::RAM_BASE..=ram::RAM_END => Ok(self.ram.read(address, size)),
            0x0020_0000..=0x7f_ffff => {
                let address = address & 0x1f_ffff; // Truncate to 21 bits
                Ok(self.ram.read(address, size))
            }
            rom::ROM_BASE..=rom::ROM_END => Ok(self.rom.read(address, size)),
            0x1f801810..=0x1f801817 => {
                // GPU registers
                assert!(
                    size == AccessSize::Word,
                    "[Bus] Unimplemented read size ({size:?}) for GPU registers"
                );

                Ok(self.gpu.read(address))
            }
            0x1f801040..=0x1f80104f => Ok(self.joy.read(address)),
            BIU_CONTROL_BASE..=BIU_CONTROL_END => {
                assert!(
                    size == AccessSize::Word,
                    "[Bus] Unimplemented read size ({size:?}) for memory control registers"
                );

                let index = (address - BIU_CONTROL_BASE) as usize / 4;
                Ok(self.biu_control[index])
            }
            0x1f80_1080..=0x1f80_10f4 => {
                Ok(self.dma.read(address - 0x1f80_1080, size))
            }
            0x1f80_1100..=0x1f80112f => Ok(self.timers.read(address)),
            0x1f80_1800..=0x1f80_1803 => Ok(self.cdrom.read(address, size)),
            0x1f801c00..=0x1f802000 => Ok(self.spu.read(address, size)),
            DRAM_CONTROL_BASE..=DRAM_CONTROL_END => {
                assert!(
                    size == AccessSize::Word,
                    "[Bus] Unimplemented read size ({size:?}) for DRAM control register"
                );

                Ok(self.dram_control)
            }
            INTERRUPTS_BASE..=INTERRUPTS_END => {
                Ok(self.interrupts.read(address, size))
            }
            _ => {
                println!("[Bus] Read error: address {address:#x} out of range");
                Err(())
            }
        }
    }

    /// Performs a write operation on the bus.
    pub fn write(
        &mut self,
        address: u32,
        value: u32,
        size: AccessSize,
    ) -> Result<(), ()> {
        for &(start, end, name) in IO_STUBS {
            if address >= start && address <= end {
                println!(
                    "[{name}] Writing {value:x} ({size:?}) at {address:08x}"
                );
                return Ok(());
            }
        }

        match address {
            ram::RAM_BASE..=ram::RAM_END => {
                self.ram.write(address, value, size)
            }
            0x0020_0000..=0x7f_ffff => {
                let address = address & 0x1f_ffff; // Truncate to 21 bits
                self.ram.write(address, value, size)
            }
            rom::ROM_BASE..=rom::ROM_END => {
                self.rom.write(address, value, size)
            }
            0x1f801c00..=0x1f802000 => {
                self.spu.write(address, value, size);
            }
            0x1f801040..=0x1f80104f => {
                self.joy.write(address, value);
            }
            0x1f80_0000..=0x1f80_03ff => {
                // Scratchpad RAM
                self.scratchpad.write(address, value, size);
            }
            0x1f80_1080..=0x1f80_10f4 => {
                self.dma.write(address - 0x1f80_1080, value, size);
                self.handle_dma_write();
            }
            0x1f80_1100..=0x1f80112f => {
                self.timers.write(address, value);
            }
            0x1f80_1800..=0x1f80_1803 => self.cdrom.write(address, value),
            0x1f80_1810..=0x1f80_1817 => {
                // GPU registers
                assert!(
                    size == AccessSize::Word,
                    "[Bus] Unimplemented read size ({size:?}) for GPU registers"
                );

                self.gpu.write(address, value);
            }
            0x1f80_4000 => print!("{}", value as u8 as char),
            BIU_CONTROL_BASE..=BIU_CONTROL_END => {
                assert!(size == AccessSize::Word);

                let index = (address - BIU_CONTROL_BASE) as usize / 4;
                self.biu_control[index] = value;
            }
            DRAM_CONTROL_BASE..=DRAM_CONTROL_END => {
                assert!(size == AccessSize::Word);

                self.dram_control = value;
            }
            INTERRUPTS_BASE..=INTERRUPTS_END => {
                self.interrupts.write(address, value, size);
            }
            _ => {
                println!(
                    "[Bus] Write error: {value:x} @ address {address:#x} out of range"
                );
                return Err(());
            }
        }

        Ok(())
    }

    fn handle_dma_write(&mut self) {
        if let Some(active_channel) = self.dma.active_channel() {
            let step = active_channel.step();
            let mut addr = active_channel.base();

            let (blocks, block_size) = active_channel.transfer_size();

            match active_channel.sync_mode() {
                SyncMode::Immediate => match active_channel.link() {
                    ChannelLink::Otc => {
                        let mut remaining_words = block_size;
                        // println!(
                        //     "[DMA6] OTC -> RAM @ 0x{:08x}, block, count: 0x{:04x}\n",
                        //     addr, remaining_words
                        // );
                        while remaining_words > 0 {
                            match active_channel.direction() {
                                Direction::FromRam => {
                                    panic!("Cannot OTC from RAM");
                                }
                                Direction::ToRam => {
                                    let word = match remaining_words {
                                        1 => 0xff_ffff,
                                        _ => {
                                            addr.wrapping_add(step as u32)
                                                & 0x1f_fffc
                                        }
                                    };
                                    self.ram.write(
                                        addr,
                                        word,
                                        AccessSize::Word,
                                    );
                                }
                            }
                            addr = addr.wrapping_add(step as u32) & 0x1f_fffc;
                            remaining_words -= 1;
                        }
                        active_channel.done();
                        self.dma.irq(6);
                    }
                    ChannelLink::Cdrom => {
                        let mut remaining_words = block_size * blocks;
                        while remaining_words > 0 {
                            match active_channel.direction() {
                                Direction::ToRam => {
                                    // println!(
                                    //     "[DMA2] CDROM -> RAM @ 0x{:08x}, remaining: {}",
                                    //     addr, remaining_words
                                    // );
                                    let value = self
                                        .cdrom
                                        .read(0x1f80_1802, AccessSize::Byte)
                                        | self.cdrom.read(
                                            0x1f80_1802,
                                            AccessSize::Byte,
                                        ) << 8
                                        | self.cdrom.read(
                                            0x1f80_1802,
                                            AccessSize::Byte,
                                        ) << 16
                                        | self.cdrom.read(
                                            0x1f80_1802,
                                            AccessSize::Byte,
                                        ) << 24;

                                    // println!("[dMa2] Read value: {value:08x}");

                                    self.ram.write(
                                        addr,
                                        value,
                                        AccessSize::Word,
                                    );
                                    addr = addr.wrapping_add(4);
                                    remaining_words -= 1;
                                }
                                Direction::FromRam => {
                                    panic!("Writing to CDROM? Not happening");
                                }
                            }
                        }
                        active_channel.done();
                        self.dma.irq(3);
                    }
                    _ => {
                        panic!(
                            "Cannot handle link {:?}",
                            active_channel.link()
                        );
                    }
                },
                SyncMode::LinkedList => {
                    match active_channel.link() {
                        ChannelLink::Gpu => {
                            loop {
                                match active_channel.direction() {
                                    Direction::FromRam => {
                                        let header = self
                                            .ram
                                            .read(addr, AccessSize::Word);
                                        let word_count = header >> 24;

                                        // if word_count > 0 {
                                        //     println!("[DMA2] GPU <- RAM @ 0x{:08x}, count: {},
                                        // nextAddr: 0x{:08x}",
                                        //     addr, word_count, header);
                                        // }

                                        for _ in 0..word_count {
                                            addr =
                                                addr.wrapping_add(step as u32);
                                            let cmd = self
                                                .ram
                                                .read(addr, AccessSize::Word);
                                            self.gpu.write(0x1f80_1810, cmd);
                                        }

                                        addr = header & 0xffffff;
                                        if addr == 0xffffff {
                                            break;
                                        }
                                    }
                                    Direction::ToRam => {
                                        panic!("Cannot DMA2-GPU to ram");
                                    }
                                }
                            }
                            active_channel.done();
                            self.dma.irq(2);
                        }
                        _ => {
                            panic!(
                                "Linked list is for gpu only. Found: {:?}",
                                active_channel.link()
                            );
                        }
                    }
                }
                SyncMode::Sync => match active_channel.link() {
                    ChannelLink::Gpu => {
                        for _ in 0..(blocks * block_size) as usize {
                            match active_channel.direction() {
                                Direction::FromRam => {
                                    let value =
                                        self.ram.read(addr, AccessSize::Word);
                                    self.gpu.write(0x1f80_1810, value);
                                    addr = addr.wrapping_add(step as u32);
                                }
                                Direction::ToRam => {
                                    panic!("Cannot DMA2-GPU to ram");
                                }
                            }
                        }
                        active_channel.done();
                        self.dma.irq(2);
                    }
                    ChannelLink::Spu => {
                        for _ in 0..(blocks * block_size) as usize {
                            match active_channel.direction() {
                                Direction::FromRam => {
                                    let value =
                                        self.ram.read(addr, AccessSize::Word);
                                    self.spu.write(
                                        0x1f801da8,
                                        value,
                                        AccessSize::Word,
                                    );
                                    addr = addr.wrapping_add(step as u32);
                                }
                                Direction::ToRam => {
                                    panic!("Cannot DMA4-SPU to ram");
                                }
                            }
                        }
                        active_channel.done();
                        self.dma.irq(4);
                    }
                    _ => {
                        unimplemented!(
                            "[DMA] new channel Found: {:?}",
                            active_channel.link()
                        );
                    }
                },
                _ => {
                    println!(
                        "Unhandled sync mode {:?}",
                        active_channel.sync_mode()
                    );
                    active_channel.done();
                }
            };
        }
    }
}
