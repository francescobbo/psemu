use super::Cpu;
use crate::bus::AccessSize;

#[derive(Debug)]
pub enum MemoryError {
    AlignmentError,
    BusError,
}

#[derive(Debug, PartialEq)]
enum MipsSegment {
    Kuseg,
    Kseg0,
    Kseg1,
    Kseg2,
}

impl Cpu {
    /// Performs a memory read operation.
    pub fn read_memory(&self, address: u32, size: AccessSize) -> Result<u32, MemoryError> {
        Self::check_alignment(address, size)?;

        if self.cop0.isolate_cache() && address < 0x1000 {
            // The I-Cache is mounted to the first 4KB of memory, but unimplemented
            return Ok(0);
        }

        let (segment, phys_addr) = self.extract_segment(address);

        match segment {
            MipsSegment::Kseg2 => {
                // Most of kseg2 is unmapped
                match address {
                    0xfffe_0000..=0xfffe_001f
                    | 0xfffe_0100..=0xfffe_012f
                    | 0xfffe_0134..=0xfffe_013f => {
                        // These addresses are reserved for CPU control
                        // registers, but their exact behavior is unknown.
                        // Return all bits set to 1.
                        println!(
                            "[Cpu] Unimplemented read from reserved address {address:#x} in Kseg2"
                        );
                        Ok(0xffffffff)
                    }
                    0xfffe_0130..=0xfffe_0133 => {
                        assert!(
                            size == AccessSize::Word && address & 0x3 == 0,
                            "[Cpu] Unimplemented write size ({size:?}) for BIU/Cache Control Register"
                        );

                        // This is the BIU/Cache Control Register
                        Ok(self.biu_cache_control)
                    }
                    _ => Err(MemoryError::BusError),
                }
            }
            _ => self
                .bus
                .read(phys_addr, size)
                .map_err(|_| MemoryError::BusError),
        }
    }

    /// Performs a memory write operation.
    pub fn write_memory(
        &mut self,
        address: u32,
        value: u32,
        size: AccessSize,
    ) -> Result<(), MemoryError> {
        Self::check_alignment(address, size)?;

        if self.cop0.isolate_cache() && address < 0x1000 {
            // The I-Cache is mounted to the first 4KB of memory, but unimplemented
            return Ok(());
        }

        let (segment, phys_addr) = self.extract_segment(address);

        match segment {
            MipsSegment::Kseg2 => {
                // Most of kseg2 is unmapped
                match address {
                    0xfffe_0000..=0xfffe_001f
                    | 0xfffe_0100..=0xfffe_012f
                    | 0xfffe_0134..=0xfffe_013f => {
                        // These addresses are reserved for CPU control
                        // registers, but their exact behavior is unknown.
                        // Ignore writes to them.
                        println!(
                            "[Cpu] Unimplemented write to reserved address {address:#x} in Kseg2"
                        );
                        Ok(())
                    }
                    0xfffe_0130..=0xfffe_0133 => {
                        assert!(
                            size == AccessSize::Word && address & 0x3 == 0,
                            "[Cpu] Unimplemented write size ({size:?}) for BIU/Cache Control Register"
                        );

                        // This is the BIU/Cache Control Register
                        self.biu_cache_control = value;
                        Ok(())
                    }
                    _ => Err(MemoryError::BusError),
                }
            }
            _ => self
                .bus
                .write(phys_addr, value, size)
                .map_err(|_| MemoryError::BusError),
        }
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

    /// Extracts the segment and offset from a given virtual address.
    fn extract_segment(&self, address: u32) -> (MipsSegment, u32) {
        if address & 0x8000_0000 == 0 {
            (MipsSegment::Kuseg, address)
        } else {
            // Look at the top 3 bits of the address to determine the segment
            match (address >> 28) & 0x0e {
                // Kseg0: 0x8000_0000 - 0x9fff_ffff
                0x8 => (MipsSegment::Kseg0, address & 0x1fff_ffff),
                // Kseg1: 0xa000_0000 - 0xbfff_ffff
                0xa => (MipsSegment::Kseg1, address & 0x1fff_ffff),
                // Kseg2: 0xc000_0000 - 0xffff_ffff
                0xc => (MipsSegment::Kseg2, address & 0x3fff_ffff),
                0xe => (MipsSegment::Kseg2, address & 0x3fff_ffff),
                _ => unreachable!(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::Cpu;

    #[test]
    fn test_extract_segment() {
        let cpu = Cpu::new();
        assert_eq!(
            cpu.extract_segment(0x0000_0000),
            (MipsSegment::Kuseg, 0x0000_0000)
        );
        assert_eq!(
            cpu.extract_segment(0x8000_0000),
            (MipsSegment::Kseg0, 0x0000_0000)
        );
        assert_eq!(
            cpu.extract_segment(0x9fff_ffff),
            (MipsSegment::Kseg0, 0x1fff_ffff)
        );
        assert_eq!(
            cpu.extract_segment(0xa000_0000),
            (MipsSegment::Kseg1, 0x0000_0000)
        );
        assert_eq!(
            cpu.extract_segment(0xbfff_ffff),
            (MipsSegment::Kseg1, 0x1fff_ffff)
        );
        assert_eq!(
            cpu.extract_segment(0xc000_0000),
            (MipsSegment::Kseg2, 0x0000_0000)
        );
        assert_eq!(
            cpu.extract_segment(0xe000_0000),
            (MipsSegment::Kseg2, 0x2000_0000)
        );
        assert_eq!(
            cpu.extract_segment(0xffff_ffff),
            (MipsSegment::Kseg2, 0x3fff_ffff)
        );
    }
}
