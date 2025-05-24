use crate::{
    ram::Ram,
    rom::{self, Rom},
};

#[derive(Debug)]
/// Represents the bus that connects the CPU to the rest of the system.
/// Due to Rust's ownership model, the bus will own the RAM and all the other
/// devices.
pub struct Bus {
    pub ram: Ram,
    pub rom: Rom,

    /// The memory control registers.
    /// - 0: Exp. 1 address.
    /// - 1: Exp. 2 address.
    /// - 2: Exp. 1 size and timings.
    /// - 3: Exp. 3 size and timings.
    /// - 4: ROM size and timings.
    /// - 5: SPU size and timings.
    /// - 6: CDROM size and timings.
    /// - 7: Exp. 2 size and timings.
    /// - 8: Common timings.
    memory_control: [u32; 9],

    /// The RAM size register
    ram_size: u32,
}

#[derive(Debug)]
/// Represents an error reported by the bus.
pub enum AccessError {
    // Unalined access to a 32-bit word.
    AlignmentError,

    // Access to an invalid address.
    BusError,
}

const RAM_BASE: u32 = 0x0000_0000;
const RAM_SIZE: u32 = 2 * 1024 * 1024;
const RAM_END: u32 = RAM_BASE + RAM_SIZE - 1;

const MEMORY_CONTROL_BASE: u32 = 0x1f80_1000;
const MEMORY_CONTROL_SIZE: u32 = 9 * 4; // 9 registers, each 4 bytes
const MEMORY_CONTROL_END: u32 = MEMORY_CONTROL_BASE + MEMORY_CONTROL_SIZE - 1;

const RAM_SIZE_BASE: u32 = 0x1f80_1060;
const RAM_SIZE_SIZE: u32 = 4; // 4 bytes for the RAM size register
const ROM_SIZE_END: u32 = RAM_SIZE_BASE + RAM_SIZE_SIZE - 1;

impl Bus {
    /// Creates a new system bus.
    pub fn new() -> Self {
        Self {
            ram: Ram::new(),
            rom: Rom::new(),
            memory_control: [0; 9],
            ram_size: 0,
        }
    }

    /// Performs a read operation on the bus.
    pub fn read(&self, address: u32, size: usize) -> Result<u32, AccessError> {
        Self::check_alignment(address, size)?;

        match address {
            RAM_BASE..=RAM_END => Ok(self.ram.read(address, size)),
            rom::ROM_BASE..=rom::ROM_END => Ok(self.rom.read(address, size)),
            MEMORY_CONTROL_BASE..=MEMORY_CONTROL_END => {
                assert!(
                    size == 4,
                    "[Bus] Unimplemented read size ({size}b) for memory control registers"
                );

                let index = (address - MEMORY_CONTROL_BASE) as usize / 4;
                Ok(self.memory_control[index])
            }
            RAM_SIZE_BASE..=ROM_SIZE_END => {
                assert!(
                    size == 4,
                    "[Bus] Unimplemented read size ({size}b) for RAM size register"
                );

                Ok(self.ram_size)
            }
            _ => {
                println!("[Bus] Read error: address {address:#x} out of range");
                Err(AccessError::BusError)
            }
        }
    }

    /// Performs a write operation on the bus.
    pub fn write(&mut self, address: u32, value: u32, size: usize) -> Result<(), AccessError> {
        Self::check_alignment(address, size)?;

        match address {
            RAM_BASE..=RAM_END => self.ram.write(address, value, size),
            0x1f80_4000 => print!("{}", value as u8 as char),
            MEMORY_CONTROL_BASE..=MEMORY_CONTROL_END => {
                assert!(
                    size == 4,
                    "[Bus] Unimplemented write size ({size}b) for memory control registers"
                );

                let index = (address - MEMORY_CONTROL_BASE) as usize / 4;
                self.memory_control[index] = value;
            }
            RAM_SIZE_BASE..=ROM_SIZE_END => {
                assert!(
                    size == 4,
                    "[Bus] Unimplemented read size ({size}b) for RAM size register"
                );

                self.ram_size = value;
            }
            rom::ROM_BASE..=rom::ROM_END => self.rom.write(address, value, size),
            _ => {
                println!("[Bus] Write error: {value:x} address {address:#x} out of range");
                return Ok(());
                return Err(AccessError::BusError);
            }
        }

        Ok(())
    }

    /// Checks if the address is aligned for the given size.
    /// Returns an error if the address is not aligned.
    fn check_alignment(address: u32, size: usize) -> Result<(), AccessError> {
        match size {
            1 => Ok(()),
            2 => {
                if address & 1 == 0 {
                    Ok(())
                } else {
                    println!("[Bus] Alignment error: address {address:#x} not aligned for 2 bytes");
                    Err(AccessError::AlignmentError)
                }
            }
            4 => {
                if address & 3 == 0 {
                    Ok(())
                } else {
                    println!("[Bus] Alignment error: address {address:#x} not aligned for 4 bytes");
                    Err(AccessError::AlignmentError)
                }
            }
            _ => unreachable!("[Bus] Invalid operation size: {}", size),
        }
    }
}
