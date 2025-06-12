use crate::bus::AccessSize;

pub const ROM_BASE: u32 = 0x1fc0_0000;
pub const ROM_SIZE: usize = 512 * 1024;
pub const ROM_END: u32 = ROM_BASE + (ROM_SIZE as u32) - 1;

/// Represents the PlayStation BIOS ROM.
#[derive(Debug)]
pub struct Rom {
    /// The ROM data.
    data: Vec<u8>,
}

impl Rom {
    /// Creates a new ROM instance.
    pub fn new() -> Self {
        Rom {
            data: vec![0; ROM_SIZE],
        }
    }

    /// Loads the ROM data from a vector of bytes.
    pub fn load(&mut self, data: Vec<u8>) {
        if data.len() != ROM_SIZE {
            panic!(
                "[Rom] Invalid ROM size: expected {} bytes, got {} bytes",
                ROM_SIZE,
                data.len()
            );
        }

        self.data = data;
    }

    /// Reads 1, 2 or 4 bytes from the ROM at the given address.
    pub fn read(&self, address: u32, size: AccessSize) -> u32 {
        let address = address - ROM_BASE;

        match size {
            AccessSize::Byte => self.read8(address) as u32,
            AccessSize::HalfWord => self.read16(address) as u32,
            AccessSize::Word => self.read32(address),
        }
    }

    /// Writes to the ROM at the given address. This is a no-op, ROMs are read-only.
    pub fn write(&mut self, address: u32, _: u32, size: AccessSize) {
        println!("[Rom] Write to {address:#x} with size {size:?} ignored");
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn test_rom_out_of_bounds_read8() {
        let rom = Rom::new();
        rom.read8(ROM_SIZE as u32);
    }

    #[test]
    #[should_panic]
    fn test_rom_out_of_bounds_read16() {
        let rom = Rom::new();
        rom.read16((ROM_SIZE - 1) as u32);
    }

    #[test]
    #[should_panic]
    fn test_rom_out_of_bounds_read32() {
        let rom = Rom::new();
        rom.read32((ROM_SIZE - 3) as u32);
    }
}
