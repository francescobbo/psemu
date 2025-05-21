use crate::bus::AccessError;
use crate::cpu::Cpu;

#[derive(Debug, PartialEq)]
enum MipsSegment {
    Kuseg,
    Kseg0,
    Kseg1,
    Kseg2,
}

impl Cpu {
    /// Performs a memory read operation.
    pub fn read_memory(&self, address: u32, size: usize) -> Result<u32, AccessError> {
        let (segment, phys_addr) = self.extract_segment(address);

        match segment {
            MipsSegment::Kseg2 => {
                unimplemented!("We'll handle kseg2 later")
            }
            _ => self.bus.read(phys_addr, size),
        }
    }

    /// Performs a memory write operation.
    pub fn write_memory(
        &mut self,
        address: u32,
        value: u32,
        size: usize,
    ) -> Result<(), AccessError> {
        let (segment, phys_addr) = self.extract_segment(address);

        match segment {
            MipsSegment::Kseg2 => {
                unimplemented!("We'll handle kseg2 later")
            }
            _ => self.bus.write(phys_addr, value, size),
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
