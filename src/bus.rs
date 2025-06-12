use crate::{
    AccessSize,
    gpu::Gpu,
    interrupts::{INTERRUPTS_BASE, INTERRUPTS_END, InterruptController},
    ram::{self, Ram},
    rom::{self, Rom},
    scratchpad::Scratchpad,
};

/// Represents the bus that connects the CPU to the rest of the system.
/// Due to Rust's ownership model, the bus will own the RAM and all the other
/// devices.
pub struct Bus {
    pub ram: Ram,
    pub rom: Rom,
    pub gpu: Gpu,

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
    (0x1f801080, 0x1f8010ff, "DMA"),
    (0x1f801100, 0x1f80112f, "Timers"),
    (0x1f801800, 0x1f801803, "CD-ROM"),
    (0x1f801820, 0x1f801827, "MDEC"),
    (0x1f801c00, 0x1f801fff, "SPU"),
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
            rom::ROM_BASE..=rom::ROM_END => {
                self.rom.write(address, value, size)
            }
            0x1f801040..=0x1f80104f => {
                self.joy.write(address, value);
            }
            0x1f80_0000..=0x1f80_03ff => {
                // Scratchpad RAM
                self.scratchpad.write(address, value, size);
            }
            0x1f801810..=0x1f801817 => {
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
}
