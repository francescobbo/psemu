use super::Cpu;
use crate::bus::AccessSize;

#[derive(Debug)]
pub enum MemoryError {
    AlignmentError,
    BusError,
}

impl Cpu {
    /// Performs a memory read operation.
    pub fn read_memory(&self, address: u32, size: AccessSize) -> Result<u32, MemoryError> {
        Self::check_alignment(address, size)?;

        self.bus
            .read(address, size)
            .map_err(|_| MemoryError::BusError)
    }

    /// Performs a memory write operation.
    pub fn write_memory(
        &mut self,
        address: u32,
        value: u32,
        size: AccessSize,
    ) -> Result<(), MemoryError> {
        Self::check_alignment(address, size)?;

        self.bus
            .write(address, value, size)
            .map_err(|_| MemoryError::BusError)
    }

    /// Checks if the address is aligned for the given size.
    /// Returns a MemoryError if the address is not aligned.
    fn check_alignment(address: u32, size: AccessSize) -> Result<(), MemoryError> {
        match size {
            AccessSize::Byte => Ok(()), // Bytes are always aligned
            AccessSize::HalfWord => {
                if address & 1 == 0 {
                    Ok(())
                } else {
                    Err(MemoryError::AlignmentError)
                }
            }
            AccessSize::Word => {
                if address & 3 == 0 {
                    Ok(())
                } else {
                    Err(MemoryError::AlignmentError)
                }
            }
        }
    }
}
