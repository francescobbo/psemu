use crate::{
    ram::{self, Ram},
    rom::{self, Rom},
};

/// Represents the possible access sizes for memory operations.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AccessSize {
    Byte,
    HalfWord,
    Word,
}

/// Represents the bus that connects the CPU to the rest of the system.
/// Due to Rust's ownership model, the bus will own the RAM and all the other
/// devices.
pub struct Bus {
    pub ram: Ram,
    pub rom: Rom,

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
}

const BIU_CONTROL_BASE: u32 = 0x1f80_1000;
const BIU_CONTROL_SIZE: u32 = 9 * 4; // 9 registers, each 4 bytes
const BIU_CONTROL_END: u32 = BIU_CONTROL_BASE + BIU_CONTROL_SIZE - 1;

const DRAM_CONTROL_BASE: u32 = 0x1f80_1060;
const DRAM_CONTROL_SIZE: u32 = 4; // 4 bytes for the DRAM control register
const DRAM_CONTROL_END: u32 = DRAM_CONTROL_BASE + DRAM_CONTROL_SIZE - 1;

impl Bus {
    /// Creates a new system bus.
    pub fn new() -> Self {
        Self {
            ram: Ram::new(),
            rom: Rom::new(),
            biu_control: [0; 9],
            dram_control: 0,
        }
    }

    /// Performs a read operation on the bus.
    pub fn read(&self, address: u32, size: AccessSize) -> Result<u32, ()> {
        match address {
            ram::RAM_BASE..=ram::RAM_END => Ok(self.ram.read(address, size)),
            rom::ROM_BASE..=rom::ROM_END => Ok(self.rom.read(address, size)),
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
            _ => {
                println!("[Bus] Read error: address {address:#x} out of range");
                Err(())
            }
        }
    }

    /// Performs a write operation on the bus.
    pub fn write(&mut self, address: u32, value: u32, size: AccessSize) -> Result<(), ()> {
        match address {
            ram::RAM_BASE..=ram::RAM_END => self.ram.write(address, value, size),
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
            rom::ROM_BASE..=rom::ROM_END => self.rom.write(address, value, size),
            _ => {
                println!("[Bus] Write error: {value:x} @ address {address:#x} out of range");
                return Err(());
            }
        }

        Ok(())
    }
}
