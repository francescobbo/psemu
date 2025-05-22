use crate::ram::Ram;

#[derive(Debug)]
/// Represents the bus that connects the CPU to the rest of the system.
/// Due to Rust's ownership model, the bus will own the RAM and all the other
/// devices.
pub struct Bus {
    pub ram: Ram,
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

impl Bus {
    /// Creates a new bus with a 2MB RAM.
    pub fn new() -> Self {
        Self { ram: Ram::new() }
    }

    /// Performs a read operation on the bus.
    pub fn read(&self, address: u32, size: usize) -> Result<u32, AccessError> {
        Self::check_alignment(address, size)?;

        match address {
            RAM_BASE..=RAM_END => Ok(self.ram.read(address, size)),
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
            _ => {
                println!("[Bus] Write error: address {address:#x} out of range");
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
