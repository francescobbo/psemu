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
        todo!("Implement read16");
    }

    fn read32(&self, address: u32) -> u32 {
        todo!("Implement read32");
    }

    fn write8(&mut self, address: u32, value: u8) {
        self.data[address as usize] = value;
    }

    fn write16(&mut self, address: u32, value: u16) {
        todo!("Implement write16");
    }

    fn write32(&mut self, address: u32, value: u32) {
        todo!("Implement write32");
    }
    //] ram-accessors
}
//] ram-new
