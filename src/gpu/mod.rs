pub struct Gpu {
    fifo: [u32; 2000], // FIFO for GPU commands
    fifo_index: usize, // Current index in the FIFO
    vram: Vec<u16>,    // Video RAM for storing pixel data
    is_reading: usize,
}

impl Gpu {
    pub fn new() -> Self {
        let vram = vec![0; 1024 * 512]; // Initialize VRAM with 1024x512 pixels, each pixel is 16 bits (RGB565)

        Gpu {
            fifo: [0; 2000],
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
            0x00..=0x08 => 1,
            0x28 => 5,
            0x2c => 9,
            0x30 => 6,
            0x38 => 8,
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
            0x20 | 0x22 | 0x28 | 0x2a => {
                let opaque = command & 2 == 0;

                // Flat-shaded 3/4 point polygon. Opaque/semi-transparent.
                let c0 = self.fifo[0] & 0xffffff;
                let v0 = self.fifo[1];
                let v1 = self.fifo[2];
                let v2 = self.fifo[3];

                self.shaded_triangle(c0, v0, c0, v1, c0, v2, opaque);

                if (command & 0x8) == 0x8 {
                    println!(
                        "[GPU] Drawing 4-point polygon with command {:#x}",
                        command
                    );
                    let v3 = self.fifo[4];

                    self.shaded_triangle(c0, v1, c0, v2, c0, v3, opaque);
                }
            }
            0x30 | 0x32 | 0x38 | 0x3a => {
                let opaque = command & 2 == 0;

                // Shaded 3/4 point polygon. Opaque/semi-transparent.
                let c0 = self.fifo[0] & 0xffffff;
                let v0 = self.fifo[1];
                let c1 = self.fifo[2] & 0xffffff;
                let v1 = self.fifo[3];
                let c2 = self.fifo[4] & 0xffffff;
                let v2 = self.fifo[5];

                self.shaded_triangle(c0, v0, c1, v1, c2, v2, opaque);

                if command & 8 == 0x8 {
                    let c3 = self.fifo[6] & 0xffffff;
                    let v3 = self.fifo[7];

                    self.shaded_triangle(c1, v1, c2, v2, c3, v3, opaque);
                }
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

        // Extract RGB components from the 24-bit RGB value, as 5-5-5 format
        let r = ((rgb >> 3) & 0x1f) as u16;
        let g = ((rgb >> 11) & 0x1f) as u16;
        let b = ((rgb >> 19) & 0x1f) as u16;

        // Store the pixel in VRAM as 16-bit BGR555 format
        self.vram[y * 1024 + x] = r | (g << 5) | (b << 10);
    }

    fn shaded_triangle(
        &mut self,
        c0: u32,
        v0: u32,
        c1: u32,
        v1: u32,
        c2: u32,
        v2: u32,
        opaque: bool,
    ) {
        // This function should implement the logic to draw a shaded triangle
        // using the provided vertex colors and coordinates.
        // For now, we will just print the values.

        // Convert colors to RGB888 format
        let (r0, g0, b0) = Self::command_to_rgb888(c0);
        let (r1, g1, b1) = Self::command_to_rgb888(c1);
        let (r2, g2, b2) = Self::command_to_rgb888(c2);

        // Convert vertex coordinates to (x, y) pairs
        let (x0, y0) = Self::command_to_coords(v0);
        let (x1, y1) = Self::command_to_coords(v1);
        let (x2, y2) = Self::command_to_coords(v2);

        println!(
            "[GPU] Drawing shaded triangle: ({}, {}) - ({}, {}) - ({}, {}) with colors: ({}, {}, {}) - ({}, {}, {}) - ({}, {}, {})",
            x0, y0, x1, y1, x2, y2, r0, g0, b0, r1, g1, b1, r2, g2, b2
        );

        let min_x = x0.min(x1).min(x2) as usize;
        let max_x = x0.max(x1).max(x2) as usize;
        let min_y = y0.min(y1).min(y2) as usize;
        let max_y = y0.max(y1).max(y2) as usize;

        let area = Self::edge_fn(
            x0 as f64, y0 as f64, x1 as f64, y1 as f64, x2 as f64, y2 as f64,
        );

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let px = x as f64 + 0.5;
                let py = y as f64 + 0.5;

                let w0 = Self::edge_fn(
                    x1 as f64, y1 as f64, x2 as f64, y2 as f64, px, py,
                );
                let w1 = Self::edge_fn(
                    x2 as f64, y2 as f64, x0 as f64, y0 as f64, px, py,
                );
                let w2 = Self::edge_fn(
                    x0 as f64, y0 as f64, x1 as f64, y1 as f64, px, py,
                );

                if (w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0)
                    || (w0 <= 0.0 && w1 <= 0.0 && w2 <= 0.0)
                {
                    let alpha = w0 / area;
                    let beta = w1 / area;
                    let gamma = w2 / area;

                    let r = (alpha * r0 as f64
                        + beta * r1 as f64
                        + gamma * r2 as f64)
                        .clamp(0.0, 255.0) as u8;

                    let g = (alpha * g0 as f64
                        + beta * g1 as f64
                        + gamma * g2 as f64)
                        .clamp(0.0, 255.0) as u8;

                    let b = (alpha * b0 as f64
                        + beta * b1 as f64
                        + gamma * b2 as f64)
                        .clamp(0.0, 255.0) as u8;

                    let bgr555 = Self::rgb888_to_bgr555(r, g, b);
                    self.vram[y * 1024 + x] = bgr555;
                }
            }
        }
    }

    fn edge_fn(x0: f64, y0: f64, x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
        (x1 - x0) * (y2 - y0) - (y1 - y0) * (x2 - x0)
    }

    fn command_to_rgb888(command: u32) -> (u8, u8, u8) {
        let r = ((command >> 0) & 0xff) as u8;
        let g = ((command >> 8) & 0xff) as u8;
        let b = ((command >> 16) & 0xff) as u8;

        (r, g, b)
    }

    fn rgb888_to_bgr555(r: u8, g: u8, b: u8) -> u16 {
        let r = (r >> 3) & 0x1f; // 5 bits for red
        let g = (g >> 3) & 0x1f; // 5 bits for green
        let b = (b >> 3) & 0x1f; // 5 bits for blue

        (r as u16) | ((g as u16) << 5) | ((b as u16) << 10)
    }

    fn command_to_coords(command: u32) -> (u16, u16) {
        let x = (command & 0x03ff) as u16;
        let y = ((command >> 16) & 0x01ff) as u16;

        (x, y)
    }
}
