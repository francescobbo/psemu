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

    pub fn read8(&self, address: u32) -> u8 {
        if address as usize >= RAM_SIZE {
            panic!("[RAM] Out of bounds read at {:#X}", address);
        }

        self.data[address as usize]
    }

    pub fn read16(&self, address: u32) -> u16 {
        if address as usize >= RAM_SIZE - 1 {
            panic!("[RAM] Out of bounds read at {:#X}", address);
        }

        let bytes = [self.data[address as usize], self.data[address as usize + 1]];

        u16::from_le_bytes(bytes)
    }

    pub fn read32(&self, address: u32) -> u32 {
        if address as usize >= RAM_SIZE - 3 {
            panic!("[RAM] Out of bounds read at {:#X}", address);
        }

        let bytes = [
            self.data[address as usize],
            self.data[address as usize + 1],
            self.data[address as usize + 2],
            self.data[address as usize + 3],
        ];

        u32::from_le_bytes(bytes)
    }

    pub fn write8(&mut self, address: u32, value: u8) {
        if address as usize >= RAM_SIZE {
            panic!("[RAM] Out of bounds write at {:#X}", address);
        }

        self.data[address as usize] = value;
    }

    pub fn write16(&mut self, address: u32, value: u16) {
        if address as usize >= RAM_SIZE - 1 {
            panic!("[RAM] Out of bounds write at {:#X}", address);
        }

        let bytes = value.to_le_bytes();

        self.data[address as usize] = bytes[0];
        self.data[address as usize + 1] = bytes[1];
    }

    pub fn write32(&mut self, address: u32, value: u32) {
        if address as usize >= RAM_SIZE - 3 {
            panic!("[RAM] Out of bounds write at {:#X}", address);
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

        ram.write8(0x0000, 0xFF);
        assert_eq!(ram.read8(0x0000), 0xFF);

        ram.write16(0x0002, 0xABCD);
        assert_eq!(ram.read16(0x0002), 0xABCD);

        ram.write32(0x0004, 0x12345678);
        assert_eq!(ram.read32(0x0004), 0x12345678);

        assert_eq!(ram.read8(0x0005), 0x56);
        assert_eq!(ram.read16(0x0004), 0x5678);
        assert_eq!(ram.read32(0x0002), 0x5678ABCD);
    }

    #[test]
    fn test_ram_out_of_bounds() {
        let result = std::panic::catch_unwind(|| {
            let ram = Ram::new();
            ram.read8(RAM_SIZE as u32);
        });
        assert!(result.is_err());

        let result = std::panic::catch_unwind(|| {
            let ram = Ram::new();
            ram.read16((RAM_SIZE - 1) as u32);
        });
        assert!(result.is_err());

        let result = std::panic::catch_unwind(|| {
            let ram = Ram::new();
            ram.read32((RAM_SIZE - 3) as u32);
        });
        assert!(result.is_err());

        let result = std::panic::catch_unwind(|| {
            let mut ram = Ram::new();
            ram.write8(RAM_SIZE as u32, 0xFF);
        });
        assert!(result.is_err());

        let result = std::panic::catch_unwind(|| {
            let mut ram = Ram::new();
            ram.write16((RAM_SIZE - 1) as u32, 0xFF);
        });
        assert!(result.is_err());

        let result = std::panic::catch_unwind(|| {
            let mut ram = Ram::new();
            ram.write32((RAM_SIZE - 3) as u32, 0xFF);
        });
        assert!(result.is_err());
    }
}
