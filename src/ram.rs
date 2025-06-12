use crate::AccessSize;
pub const RAM_SIZE: usize = 2 * 1024 * 1024;

/// A RAM (Random Access Memory) structure that simulates a memory space,
/// with little-endian accessors for reading and writing bytes.
pub struct Ram {
    data: Vec<u8>,
}

impl Ram {
    pub fn new() -> Self {
        Self {
            data: vec![0; RAM_SIZE],
        }
    }

    pub fn read(&self, address: u32, size: AccessSize) -> u32 {
        match size {
            AccessSize::Byte => self.read8(address) as u32,
            AccessSize::HalfWord => self.read16(address) as u32,
            AccessSize::Word => self.read32(address),
        }
    }

    pub fn write(&mut self, address: u32, value: u32, size: AccessSize) {
        match size {
            AccessSize::Byte => self.write8(address, value as u8),
            AccessSize::HalfWord => self.write16(address, value as u16),
            AccessSize::Word => self.write32(address, value),
        }
    }
    fn read8(&self, address: u32) -> u8 {
        self.data[address as usize]
    }

    fn read16(&self, address: u32) -> u16 {
        let bytes =
            [self.data[address as usize], self.data[address as usize + 1]];

        u16::from_le_bytes(bytes)
    }

    fn read32(&self, address: u32) -> u32 {
        let bytes = [
            self.data[address as usize],
            self.data[address as usize + 1],
            self.data[address as usize + 2],
            self.data[address as usize + 3],
        ];

        u32::from_le_bytes(bytes)
    }

    fn write8(&mut self, address: u32, value: u8) {
        self.data[address as usize] = value;
    }

    fn write16(&mut self, address: u32, value: u16) {
        let bytes = value.to_le_bytes();

        self.data[address as usize] = bytes[0];
        self.data[address as usize + 1] = bytes[1];
    }

    fn write32(&mut self, address: u32, value: u32) {
        let bytes = value.to_le_bytes();

        self.data[address as usize] = bytes[0];
        self.data[address as usize + 1] = bytes[1];
        self.data[address as usize + 2] = bytes[2];
        self.data[address as usize + 3] = bytes[3];
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ram() {
        let mut ram = Ram::new();

        ram.write(0x0000, 0xff, AccessSize::Byte);
        assert_eq!(ram.read(0x0000, AccessSize::Byte), 0xff);

        ram.write(0x0002, 0xabcd, AccessSize::HalfWord);
        assert_eq!(ram.read(0x0002, AccessSize::HalfWord), 0xabcd);

        ram.write(0x0004, 0x12345678, AccessSize::Word);
        assert_eq!(ram.read(0x0004, AccessSize::Word), 0x12345678);

        assert_eq!(ram.read(0x0005, AccessSize::Byte), 0x56);
        assert_eq!(ram.read(0x0004, AccessSize::HalfWord), 0x5678);
        assert_eq!(ram.read(0x0002, AccessSize::Word), 0x5678abcd);
    }

    #[test]
    #[should_panic]
    fn test_ram_out_of_bounds_read8() {
        let ram = Ram::new();
        ram.read(RAM_SIZE as u32, AccessSize::Byte);
    }
    #[test]
    #[should_panic]
    fn test_ram_out_of_bounds_read16() {
        let ram = Ram::new();
        ram.read((RAM_SIZE - 1) as u32, AccessSize::HalfWord);
    }

    #[test]
    #[should_panic]
    fn test_ram_out_of_bounds_read32() {
        let ram = Ram::new();
        ram.read((RAM_SIZE - 3) as u32, AccessSize::Word);
    }

    #[test]
    #[should_panic]
    fn test_ram_out_of_bounds_write8() {
        let mut ram = Ram::new();
        ram.write(RAM_SIZE as u32, 0xff, AccessSize::Byte);
    }

    #[test]
    #[should_panic]
    fn test_ram_out_of_bounds_write16() {
        let mut ram = Ram::new();
        ram.write((RAM_SIZE - 1) as u32, 0xabcd, AccessSize::HalfWord);
    }

    #[test]
    #[should_panic]
    fn test_ram_out_of_bounds_write32() {
        let mut ram = Ram::new();
        ram.write((RAM_SIZE - 3) as u32, 0x12345678, AccessSize::Word);
    }
}
