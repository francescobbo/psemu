//[ ram-new
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

    //[ ram-accessors
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
    //] ram-accessors
}
//] ram-new
