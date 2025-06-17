use crate::bus::AccessSize;

pub struct Cdrom {
    bank: u8,
    response: [u8; 16],
    response_index: usize,
    sector: Vec<u8>,
    sector_index: usize,
    int_mask: u8,
    int_status: u8,
}

impl Cdrom {
    pub fn new() -> Self {
        Cdrom {
            bank: 0, // Default bank value
            response: [0; 16], // Initialize response buffer
            response_index: 0, // Initialize response index
            sector: vec![0; 2352], // Default sector size for CD-ROM
            sector_index: 0, // Initialize sector index
            int_mask: 0, // Default interrupt mask
            int_status: 0, // Default interrupt status
        }
    }

    pub fn read(&mut self, address: u32, size: AccessSize) -> u32 {
        match address - 0x1f80_1800 {
            0 => {
                self.bank as u32 | (1 << 3)
            }
            1 => {
                // Return the response byte at the current index
                let data = self.response[self.response_index];
                self.response_index += 1;
                if self.response_index >= self.response.len() {
                    self.response_index = 0; // Reset index if it exceeds the length
                }

                data as u32
            }
            2 => {
                match size {
                    AccessSize::Byte => self.get_sector_byte() as u32,
                    AccessSize::HalfWord => {
                        let data = [
                            self.get_sector_byte(),
                            self.get_sector_byte(),
                        ];

                        i16::from_le_bytes(data) as u32
                    }
                    AccessSize::Word => {
                        let data = [
                            self.get_sector_byte(),
                            self.get_sector_byte(),
                            self.get_sector_byte(),
                            self.get_sector_byte(),
                        ];

                        u32::from_le_bytes(data)
                    }
                }
            }
            3 => {
                if self.bank == 0 || self.bank == 1 {
                    // Return the interrupt mask
                    self.int_mask as u32 | 0xe0
                } else {
                    // For other banks, return the interrupt status
                    self.int_status as u32 | 0xe0
                }
            }
            _ => unreachable!("[CDROM] Unimplemented read at address {:#x}", address),
        }
    }

    pub fn write(&mut self, address: u32, value: u32) {
        match address - 0x1f80_1800 {
            0 => {
                self.bank = (value & 0x03) as u8; // Set the bank to the lower 2 bits
            }
            1 => {
                println!("[CDROM] Write to 1 at bank {}: {:#x}", self.bank, value);
            }
            2 => {
                match self.bank {
                    1 => {
                        // Write the interrupt mask
                        self.int_mask = (value & 0x1f) as u8; // Mask to lower 5 bits
                    }
                    _ => {
                        println!("[CDROM] Write to 2 at bank {}: {:#x}", self.bank, value);
                        // For other banks, we might want to handle it differently
                        // For now, just print the value
                    }
                }
            }
            3 => {
                match self.bank {
                    1 => {
                        // Acknowledge the interrupts
                        self.int_status &= !value as u8; // Clear the bits in the interrupt status
                    }
                    _ => {
                        println!("[CDROM] Write to 3 at bank {}: {:#x}", self.bank, value);
                    }
                }
            }
            _ => unreachable!("[CDROM] Unimplemented write at address {:#x}", address),
        }
    }

    pub fn get_sector_byte(&mut self) -> u8 {
        if self.sector_index >= self.sector.len() {
            // Return 4th last byte if index exceeds length. This should be 8th for 2048-byte sectors.
            self.sector[self.sector.len() - 4]
        } else {
            let byte = self.sector[self.sector_index];
            self.sector_index += 1;
            byte
        }
    }
}