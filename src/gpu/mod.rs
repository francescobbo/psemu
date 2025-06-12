pub struct Gpu {
    fifo: [u32; 500],  // FIFO for GPU commands
    fifo_index: usize, // Current index in the FIFO
    vram: Vec<u16>,    // Video RAM for storing pixel data
    is_reading: usize,
}

impl Gpu {
    pub fn new() -> Self {
        let vram = vec![0; 1024 * 512]; // Initialize VRAM with 1024x512 pixels, each pixel is 16 bits (RGB565)

        Gpu {
            fifo: [0; 500],
            fifo_index: 0,
            vram,
            is_reading: 0,
        }
    }

    pub fn get_pixel_color(&self, i: usize) -> (u8, u8, u8) {
        let pixel = self.vram[i];

        let mut r = pixel & 0x1f; // Red channel
        let mut g = (pixel >> 5) & 0x1f; // Green channel
        let mut b = (pixel >> 10) & 0x1f; // Blue channel

        r <<= 3;
        g <<= 3;
        b <<= 3;

        (r as u8, g as u8, b as u8)
    }

    pub fn read(&mut self, address: u32) -> u32 {
        if address == 0x1f80_1814 {
            // println!("[GPU] Read GPUSTAT");
            return 0x1c00_0000;
        }

        // println!("[GPU] Read operation at address {:#x}", address);
        if self.is_reading > 0 {
            self.is_reading -= 1;
            0xffffffff
        } else {
            0
        }
    }

    pub fn write(&mut self, address: u32, value: u32) {
        // println!("[GPU] Write operation at address {:#x} with value {:#x}", address, value);

        match address {
            0x1f80_1810 => {
                // Write to GP0 register
                self.fifo[self.fifo_index] = value;
                self.fifo_index = (self.fifo_index + 1);
                if self.fifo_index == 1 {
                    // println!("[GPU] Command added to FIFO: {:#x}", value);
                }

                if self.fifo_index == self.expected_words() {
                    self.execute();
                    // Here you would handle the command based on its type
                    // For now, we just clear the FIFO
                    self.fifo_index = 0;
                }
            }
            0x1f80_1814 => {
                // Write to GPUSTAT register
                // println!("[GPU] Writing to GP1: {:#x}", value);
            }
            _ => unreachable!(),
        }
    }

    pub fn expected_words(&self) -> usize {
        let first = self.fifo[0];
        match first >> 24 {
            0xa0 => {
                if self.fifo.len() < 3 {
                    return 1000; // Not enough data in FIFO
                }

                let ys = self.fifo[2] >> 16;
                let xs = self.fifo[2] & 0xffff;

                let mut total_halfwords = (ys * xs) as usize;
                if total_halfwords & 1 == 1 {
                    total_halfwords += 1; // Round up if odd
                }

                let total_words = total_halfwords / 2;
                3 + total_words // 3 for the command and 1 for each pair of pixels
            }
            0xc0 => 3,
            0x68 => 2, // pixel command
            0xe1..=0xe6 => 1,

            _ => {
                panic!("[GPU] Unknown command for sizing: {:#x}", first);
            }
        }
    }

    pub fn execute(&mut self) {
        if self.fifo_index == 0 {
            println!("[GPU] No command to execute.");
            return;
        }

        let command = self.fifo[0] >> 24;
        // println!("[GPU] Executing command: {:#x}", command);

        match command {
            0x68 => {
                self.render_pixel();
            }
            0xa0 => {
                let dest_x = (self.fifo[1] & 0x3ff) as usize;
                let dest_y = ((self.fifo[1] >> 16) & 0x1ff) as usize;
                let mut xs = (self.fifo[2] & 0x3ff) as usize;
                let mut ys = ((self.fifo[2] >> 16) & 0x1ff) as usize;

                if xs == 0 {
                    xs = 1024
                }
                if ys == 0 {
                    ys = 512
                }

                let mut data: Vec<u16> = Vec::new();

                for word in self.fifo[3..].iter() {
                    data.push(*word as u16);
                    data.push((*word >> 16) as u16);
                }

                let mut src_idx = 0;
                let mut dst_idx = dest_y * 512 + dest_x;

                for yoffs in 0..ys {
                    for xoffs in 0..xs {
                        println!(
                            "Setting pixel {},{} to {:04x}",
                            dest_x + xoffs,
                            dest_y + yoffs,
                            data[src_idx]
                        );

                        self.vram[dst_idx + xoffs] = data[src_idx];
                        src_idx += 1;
                    }
                    dst_idx += 1024;
                }
            }
            0xc0 => {
                println!(
                    "[GPU] Reading pixels from VRAM: {:?}",
                    &self.fifo[0..self.fifo_index]
                );
                self.is_reading = 1;
            }
            _ => {}
        }
    }

    pub fn render_pixel(&mut self) {
        let rgb = self.fifo[0] & 0xffffff;

        let x = (self.fifo[1] & 0xffff) as usize;
        let y = (self.fifo[1] >> 16) as usize;

        if x >= 1024 || y >= 512 {
            println!("[GPU] Pixel coordinates out of bounds: ({}, {})", x, y);
            return;
        }

        let r = (rgb & 0xff) as u16;
        let g = ((rgb >> 8) & 0xff) as u16;
        let b = ((rgb >> 16) & 0xff) as u16;

        // Convert RGB888 to RGB565
        // RGB555 format: 5 bits for red, 5 bits for green, 5 bits for blue
        // Note: This is a simplified conversion, actual conversion may vary based on the GPU's color depth
        // Ensure the values are in the range for RGB555
        let r = r >> 3; // 5 bits for red
        let g = g >> 3; // 5 bits for green
        let b = b >> 3; // 5 bits for blue

        // Store the pixel in VRAM
        self.vram[y * 1024 + x] = r | (g << 5) | (b << 10); // RGB555 format
    }
}
