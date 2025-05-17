const RAM_SIZE: usize = 2 * 1024 * 1024;

#[derive(Debug)]
pub struct Ram {
    data: Vec<u8>,
}

impl Ram {
    pub fn new() -> Self {
        Ram {
            data: vec![0; RAM_SIZE],
        }
    }

    pub fn read(&self, address: u32, size: usize) -> u32 {
        match size {
            1 => self.read8(address) as u32,
            2 => self.read16(address) as u32,
            4 => self.read32(address),
            _ => unreachable!("[Ram] Invalid size for read: {}", size),
        }
    }

    pub fn write(&mut self, address: u32, value: u32, size: usize) {
        match size {
            1 => self.write8(address, value as u8),
            2 => self.write16(address, value as u16),
            4 => self.write32(address, value),
            _ => unreachable!("[Ram] Invalid size for write: {}", size),
        }
    }

    fn read8(&self, address: u32) -> u8 {
        if address as usize >= RAM_SIZE {
            panic!("[Ram] Out of bounds read at {address:#X}");
        }

        self.data[address as usize]
    }

    fn read16(&self, address: u32) -> u16 {
        if address as usize >= RAM_SIZE - 1 {
            panic!("[Ram] Out of bounds read at {address:#X}");
        }

        let bytes = [self.data[address as usize], self.data[address as usize + 1]];

        u16::from_le_bytes(bytes)
    }

    fn read32(&self, address: u32) -> u32 {
        if address as usize >= RAM_SIZE - 3 {
            panic!("[Ram] Out of bounds read at {address:#X}");
        }

        let bytes = [
            self.data[address as usize],
            self.data[address as usize + 1],
            self.data[address as usize + 2],
            self.data[address as usize + 3],
        ];

        u32::from_le_bytes(bytes)
    }

    fn write8(&mut self, address: u32, value: u8) {
        if address as usize >= RAM_SIZE {
            panic!("[Ram] Out of bounds write at {address:#X}");
        }

        self.data[address as usize] = value;
    }

    fn write16(&mut self, address: u32, value: u16) {
        if address as usize >= RAM_SIZE - 1 {
            panic!("[Ram] Out of bounds write at {address:#X}");
        }

        let bytes = value.to_le_bytes();

        self.data[address as usize] = bytes[0];
        self.data[address as usize + 1] = bytes[1];
    }

    fn write32(&mut self, address: u32, value: u32) {
        if address as usize >= RAM_SIZE - 3 {
            panic!("[Ram] Out of bounds write at {address:#X}");
        }

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

        ram.write(0x0000, 0xff, 1);
        assert_eq!(ram.read(0x0000, 1), 0xff);

        ram.write(0x0002, 0xabcd, 2);
        assert_eq!(ram.read(0x0002, 2), 0xabcd);

        ram.write(0x0004, 0x12345678, 4);
        assert_eq!(ram.read(0x0004, 4), 0x12345678);

        assert_eq!(ram.read(0x0005, 1), 0x56);
        assert_eq!(ram.read(0x0004, 2), 0x5678);
        assert_eq!(ram.read(0x0002, 4), 0x5678abcd);
    }

    #[test]
    #[should_panic]
    fn test_ram_out_of_bounds_read8() {
        let ram = Ram::new();
        ram.read8(RAM_SIZE as u32);
    }

    #[test]
    #[should_panic]
    fn test_ram_out_of_bounds_read16() {
        let ram = Ram::new();
        ram.read16((RAM_SIZE - 1) as u32);
    }

    #[test]
    #[should_panic]
    fn test_ram_out_of_bounds_read32() {
        let ram = Ram::new();
        ram.read32((RAM_SIZE - 3) as u32);
    }

    #[test]
    #[should_panic]
    fn test_ram_out_of_bounds_write8() {
        let mut ram = Ram::new();
        ram.write8(RAM_SIZE as u32, 0xff);
    }

    #[test]
    #[should_panic]
    fn test_ram_out_of_bounds_write16() {
        let mut ram = Ram::new();
        ram.write16((RAM_SIZE - 1) as u32, 0xabcd);
    }

    #[test]
    #[should_panic]
    fn test_ram_out_of_bounds_write32() {
        let mut ram = Ram::new();
        ram.write32((RAM_SIZE - 3) as u32, 0x12345678);
    }
}
