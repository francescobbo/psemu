use crate::ram::{self, Ram};

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
}

impl Bus {
    /// Creates a new bus with a 2MB RAM.
    pub fn new() -> Self {
        Self { ram: Ram::new() }
    }

    /// Performs a read operation on the bus.
    pub fn read(&self, address: u32, size: AccessSize) -> Result<u32, ()> {
        match address {
            ram::RAM_BASE..=ram::RAM_END => Ok(self.ram.read(address, size)),
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
            _ => {
                println!("[Bus] Write error: address {address:#x} out of range");
                return Err(());
            }
        }

        Ok(())
    }
}
