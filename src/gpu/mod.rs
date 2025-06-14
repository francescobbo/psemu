pub struct Gpu {
    fifo: [u32; 2000],  // FIFO for GPU commands
    fifo_index: usize,  // Current index in the FIFO
    pub vram: Vec<u16>, // Video RAM for storing pixel data
    is_reading: usize,
    reading_x: usize, // X coordinate for reading pixels
    reading_y: usize, // Y coordinate for reading pixels

    x_offset: isize,
    y_offset: isize,
    min_x: usize,
    min_y: usize,
    max_x: usize,
    max_y: usize,

    display_enable: bool, // Flag to enable/disable display
    // Direction of DMA transfer (0 off, 1 regular FIFO, 2 sending from CPU, 3 sending to CPU)
    dma_direction: u32,

    enable_dithering: bool, // Flag to enable dithering

    // Coordinates in VRAM where the data to send to the screen starts
    screen_x_start: usize,
    screen_y_start: usize,

    tex_page_x: usize,
    tex_page_y: usize,
    opacity_flag: usize,
    tex_page_depth: usize, // Color depth of the texture page
}

impl Gpu {
    pub fn new() -> Self {
        let vram = vec![0; 1024 * 512]; // Initialize VRAM with 1024x512 pixels, each pixel is 16 bits (RGB565)

        Gpu {
            fifo: [0; 2000],
            fifo_index: 0,
            vram,
            is_reading: 0,
            reading_x: 0,
            reading_y: 0,

            x_offset: 0,
            y_offset: 0,

            min_x: 0,
            min_y: 0,
            max_x: 1023,
            max_y: 511,

            display_enable: false, // Display is off by default
            dma_direction: 0,      // DMA is off by default
            enable_dithering: false, // Dithering is disabled by default

            screen_x_start: 0,
            screen_y_start: 0,

            tex_page_x: 0,
            tex_page_y: 0,
            opacity_flag: 0,
            tex_page_depth: 0,
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
            let px0 = self.vram[self.reading_y * 1024 + self.reading_x];
            self.reading_x += 1;
            if self.reading_x >= 1024 {
                self.reading_x = 0;
                self.reading_y += 1;
            }

            let px1 = self.vram[self.reading_y * 1024 + self.reading_x];
            self.reading_x += 1;
            if self.reading_x >= 1024 {
                self.reading_x = 0;
                self.reading_y += 1;
            }

            self.is_reading -= 1;

            px0 as u32 | (px1 as u32) << 16
        } else {
            0
        }
    }

    pub fn write(&mut self, address: u32, value: u32) {
        // println!("[GPU] Write operation at address {:#x} with value {:#x}", address, value);

        match address {
            0x1f80_1810 => {
                if value >> 24 == 0 && self.fifo_index == 0 {
                    // NOPs don't even go into the FIFO
                    return;
                }

                // Write to GP0 register
                self.fifo[self.fifo_index] = value;
                self.fifo_index += 1;

                if self.fifo_index == self.expected_words() {
                    self.execute();
                    // Here you would handle the command based on its type
                    // For now, we just clear the FIFO
                    self.fifo_index = 0;
                }
            }
            0x1f80_1814 => {
                // Write to GPUSTAT register
                let cmd = value >> 24;
                match cmd {
                    0x00 => {
                        // Reset
                        self.display_enable = false;
                        self.dma_direction = 0;
                    }
                    0x03 => {
                        self.display_enable = (value & 0x1) == 0;
                    }
                    0x04 => {
                        self.dma_direction = cmd & 3;
                    }
                    0x05 => {
                        self.screen_x_start = (value & 0x3ff) as usize; // X start position
                        self.screen_y_start = ((value >> 10) & 0x1ff) as usize; // Y start position
                    }
                    _ => {
                        // println!("[GPU] Unknown command in GP1: {:#x}", cmd);
                    }
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn expected_words(&self) -> usize {
        let first = self.fifo[0];
        match first >> 24 {
            0x01 => 1,
            0x02 => 3,
            0x03..=0x1e => 1,
            0x28 => 5,
            0x2c => 9,
            0x30 => 6,
            0x38 => 8,
            0x60 | 0x62 => 3,
            0x64..=0x67 => 4,
            0x6c..=0x6f | 0x74..=0x77 | 0x7c..=0x7f => 3,
            0x68 | 0x6a | 0x70 | 0x72 | 0x78 | 0x7a => 2,
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
            0x02 => {
                // Fill rectangle in VRAM

                let c = Self::command_to_rgb888(self.fifo[0] & 0xffffff); // Color
                let c = Self::rgb888_to_bgr555(c.0, c.1, c.2) as u16;

                let x0 = (self.fifo[1] & 0x3ff) as usize; // X position
                let y0 = ((self.fifo[1] >> 16) & 0x1ff) as usize; // Y position

                let w = (self.fifo[2] & 0x3ff) as usize; // Width
                let h = ((self.fifo[2] >> 16) & 0x1ff) as usize; // Height

                for y in 0..h {
                    for x in 0..w {
                        let idx =
                            ((y0 + y) & 0x1ff) * 1024 + ((x0 + x) & 0x3ff);
                        if idx < self.vram.len() {
                            self.vram[idx] = c;
                        } else {
                            println!(
                                "[GPU] Fill rectangle out of bounds at ({}, {})",
                                x0 + x,
                                y0 + y
                            );
                        }
                    }
                }
            }
            0xa0 => {
                println!(
                    "[GPU] Drawing pixels to VRAM: {:?}",
                    &self.fifo[0..=2]
                );

                if self.fifo[2] == 0x10010 {
                    // 1x16 is a CLUT

                    // Show the 16 pixels in the CLUT
                    // Encoded in self.fifo[3..]. Each entry is 2 u16 values

                    for i in 0..=15 {
                        let idx = 3 + i / 2;
                        let pixel = self.fifo[idx];
                        let color = if i % 2 == 0 {
                            pixel & 0xffff // First half
                        } else {
                            (pixel >> 16) & 0xffff // Second half
                        };

                        let (r, g, b) = Self::command_to_rgb888(color);
                        println!(
                            "[GPU] CLUT write pixel {}: R: {}, G: {}, B: {}",
                            i, r, g, b
                        );
                    }
                }

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
                let mut dst_idx = dest_y * 1024 + dest_x;

                for _ in 0..ys {
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
                self.reading_x = (self.fifo[1] & 0x3ff) as usize; // X position
                self.reading_y = ((self.fifo[1] >> 16) & 0x1ff) as usize; // Y position

                let size_x = (self.fifo[2] & 0x3ff) as usize; // Width
                let size_y = ((self.fifo[2] >> 16) & 0x1ff) as usize; // Height

                let total_halfwords = size_x * size_y;
                let total_words = (total_halfwords + 1) / 2; // Round up if odd

                self.is_reading = total_words;
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
                    // println!(
                    //     "[GPU] Drawing 4-point polygon with command {:#x}",
                    //     command
                    // );
                    let v3 = self.fifo[4];

                    self.shaded_triangle(c0, v1, c0, v2, c0, v3, opaque);
                }
            }
            0x2c => {
                // Similar 0x64. However:
                // - takes 4 vertices instead of 1 + (w, h)
                // - takes the texpage inline, instead of from previous E1 command (actually, this updates the state of the E1 command)
                // - uses the CLUT just like 0x64, however
                // - each vertex has it's own texture coordinates
                //
                // And, crucially, this is rendered as two triangles, not a rectangle.

                // Textured Rectangle, variable size, opaque, texture-blending
                let modulation_color = self.fifo[0] & 0xffffff; // Color
                let x0 = (self.fifo[1] & 0x3ff) as usize; // X position
                let y0 = ((self.fifo[1] >> 16) & 0x1ff) as usize; // Y position

                let x1 = (self.fifo[3] & 0x3ff) as usize; // X position
                let y1 = ((self.fifo[3] >> 16) & 0x1ff) as usize; // Y position

                let x2 = (self.fifo[5] & 0x3ff) as usize; // X position
                let y2 = ((self.fifo[5] >> 16) & 0x1ff) as usize; // Y position

                let x3 = (self.fifo[7] & 0x3ff) as usize; // X position
                let y3 = ((self.fifo[7] >> 16) & 0x1ff) as usize; // Y position

                let clut = (self.fifo[2] >> 16) as usize; // CLUT address
                let clut_x = (clut & 0x3f) * 16;
                let clut_y = (clut >> 6) & 0x1ff;

                let texpage = (self.fifo[4] >> 16) as usize; // Texture page address
                self.tex_page_x = (texpage & 0xf) * 64; // X position
                self.tex_page_y = ((texpage >> 4) & 0x1) * 256; // Y position

                let tx0 = (self.fifo[2] & 0xff) as usize; // Texture X position
                let ty0 = ((self.fifo[2] >> 8) & 0xff) as usize; // Texture Y position

                let tx1 = (self.fifo[4] & 0xff) as usize; // Texture X position
                let ty1 = ((self.fifo[4] >> 8) & 0xff) as usize; // Texture Y position

                let tx2 = (self.fifo[6] & 0xff) as usize; // Texture X position
                let ty2 = ((self.fifo[6] >> 8) & 0xff) as usize; // Texture Y position

                let tx3 = (self.fifo[8] & 0xff) as usize; // Texture X position
                let ty3 = ((self.fifo[8] >> 8) & 0xff) as usize; // Texture Y position

                self.textured_triangle(
                    modulation_color,
                    x0,
                    y0,
                    tx0,
                    ty0,
                    x1,
                    y1,
                    tx1,
                    ty1,
                    x2,
                    y2,
                    tx2,
                    ty2,
                    clut_x,
                    clut_y,
                );

                self.textured_triangle(
                    modulation_color,
                    x1,
                    y1,
                    tx1,
                    ty1,
                    x2,
                    y2,
                    tx2,
                    ty2,
                    x3,
                    y3,
                    tx3,
                    ty3,
                    clut_x,
                    clut_y,
                );
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
            0x60 => {
                // Fill rectangle in VRAM. Similar to 0x02, but:
                // - respects the offsets
                // - uses the clipping rectangle defined by min_x, min_y, max_x, max_y

                let c = Self::command_to_rgb888(self.fifo[0] & 0xffffff); // Color
                let c = Self::rgb888_to_bgr555(c.0, c.1, c.2) as u16;

                let x0 = (self.fifo[1] & 0x3ff) as usize; // X position
                let y0 = ((self.fifo[1] >> 16) & 0x1ff) as usize; // Y position

                let w = (self.fifo[2] & 0x3ff) as usize; // Width
                let h = ((self.fifo[2] >> 16) & 0x1ff) as usize; // Height

                let x0 = ((x0 as isize + self.x_offset) & 0x3ff) as usize; // Wrap around at 1024
                let y0 = ((y0 as isize + self.y_offset) & 0x1ff) as usize; // Wrap around at 512

                for y in 0..h {
                    for x in 0..w {
                        if (x0 + x) < self.min_x
                            || (x0 + x) > self.max_x
                            || (y0 + y) < self.min_y
                            || (y0 + y) > self.max_y
                        {
                            continue; // Skip pixels outside the clipping rectangle
                        }

                        let idx =
                            ((y0 + y) & 0x1ff) * 1024 + ((x0 + x) & 0x3ff);
                        self.vram[idx] = c;
                    }
                }
            }
            0x64 => {
                // Textured Rectangle, variable size, opaque, texture-blending
                let modulation_color = self.fifo[0] & 0xffffff; // Color
                let x0 = (self.fifo[1] & 0x3ff) as usize; // X position
                let y0 = ((self.fifo[1] >> 16) & 0x1ff) as usize; // Y position

                let w = (self.fifo[3] & 0x3ff) as usize; // Width
                let h = ((self.fifo[3] >> 16) & 0x1ff) as usize; // Height

                let clut = self.fifo[2] >> 16; // CLUT address
                let clut_x = (clut & 0x3f) * 16;
                let clut_y = (clut >> 6) & 0x1ff;

                let tex_x = (self.fifo[2] & 0xff) as usize; // Texture X position
                let tex_y = ((self.fifo[2] >> 8) & 0xff) as usize; // Texture Y position

                // The process is:
                // Go pixel by pixel (x0 to x0 + w, y0 to y0 + h)
                // For each pixel, sample the texture at:
                //   self.tex_page_x + tex_x + x/4 (wrap around when tex_x + x/4 >= 256)
                // Each byte in the texture has 2 4-bit indices. One byte in the texture covers 2 pixels
                //   (each u16 has 4 pixels, so we need to read 2 bytes for each pixel pair)
                // Extract the 4-bit index for the pixel from the texture
                // Use the index to get the color from the CLUT
                // Then blend the color with the color from the command

                // println!(
                //     "[GPU] Drawing textured rectangle at ({}, {}) with size {}x{} and CLUT at ({}, {}). Texture at ({}, {}), with offset ({}, {})",
                //     x0,
                //     y0,
                //     w,
                //     h,
                //     clut_x,
                //     clut_y,
                //     self.tex_page_x,
                //     self.tex_page_y,
                //     tex_x,
                //     tex_y
                // );

                for y in 0..h {
                    for x in 0..w {
                        let tex_x_pos =
                            self.tex_page_x + ((tex_x + x / 4) % 256); // Wrap around at 256
                        let tex_y_pos = self.tex_page_y + ((tex_y + y) % 512); // Wrap around at 512

                        // Get the texture byte
                        let tex_byte = self.vram[tex_y_pos * 1024 + tex_x_pos];

                        // Get the pixel index from the texture byte
                        let pixel_index =
                            ((tex_byte >> ((x % 4) * 4)) & 0xf) as usize;

                        // println!(
                        //     "[GPU] Sampling texture at ({}, {}) for pixel ({}, {}). Idx: {}",
                        //     tex_x_pos, tex_y_pos, x, y, pixel_index
                        // );

                        // Get the color from the CLUT
                        let clut_color = self.vram[(clut_y * 1024
                            + clut_x
                            + pixel_index as u32)
                            as usize];

                        if clut_color == 0 {
                            // If the CLUT color is 0, skip this pixel (transparent)
                            continue;
                        }

                        // println!(
                        //     "[GPU] CLUT color for index {}: {:#x}",
                        //     pixel_index, clut_color
                        // );

                        // Blend the color with the command color (TODO)
                        let blended_color = Self::modulate_color_555(
                            clut_color,
                            modulation_color,
                        );

                        let idx =
                            ((y0 + y) & 0x1ff) * 1024 + ((x0 + x) & 0x3ff);
                        self.vram[idx] = blended_color as u16;
                    }
                }
            }
            0x68 => {
                self.render_pixel();
            }
            0xe1 => {
                self.tex_page_x = ((self.fifo[0] & 0xf) * 64) as usize; // X position
                self.tex_page_y = (((self.fifo[0] >> 4) & 1) * 256) as usize; // Y position
                self.opacity_flag = ((self.fifo[0] >> 5) & 0x3) as usize;
                self.tex_page_depth = ((self.fifo[0] >> 7) & 0x3) as usize; // Depth
                self.enable_dithering = self.fifo[0] & 0x200 != 0;
            }
            0xe2 => {
                let mask_x = (self.fifo[0] & 0x1f);
                let mask_y = ((self.fifo[0] >> 5) & 0x1f);
                let off_x = ((self.fifo[0] >> 10) & 0x1f) as usize;
                let off_y = ((self.fifo[0] >> 15) & 0x1f) as usize;
                // println!(
                //     "Texture window settings: mask_x: {}, mask_y: {}, off_x: {}, off_y: {}",
                //     mask_x, mask_y, off_x, off_y
                // );
            }
            0xe3 => {
                // Clipping top left corner
                self.min_x = (self.fifo[0] & 0x3ff) as usize; // X position
                self.min_y = ((self.fifo[0] >> 10) & 0x1ff) as usize; // Y position
            }
            0xe4 => {
                // Clipping bottom right corner
                self.max_x = (self.fifo[0] & 0x3ff) as usize; // X position
                self.max_y = ((self.fifo[0] >> 10) & 0x1ff) as usize; // Y position
            }
            0xe5 => {
                // sign-extend x_offset from 11 bits to 16 bits
                self.x_offset =
                    ((((self.fifo[0] & 0x7ff) as i16) << 5) >> 5) as isize;

                // sign-extend y_offset from 11 bits to 16 bits
                self.y_offset = ((((self.fifo[0] >> 11) & 0x7ff) as i16) << 5
                    >> 5) as isize;
            }
            0xe6 => {
                // println!("Mask bit settings: {:#x}", self.fifo[0]);
            }
            _ => {
                println!(
                    "[GPU] Unknown command {:#x} with {} words in FIFO",
                    command, self.fifo_index
                );
            }
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

        // Convert colors to RGB888 format
        let (r0, g0, b0) = Self::command_to_rgb888(c0);
        let (r1, g1, b1) = Self::command_to_rgb888(c1);
        let (r2, g2, b2) = Self::command_to_rgb888(c2);

        // Convert vertex coordinates to (x, y) pairs
        let (x0, y0) = Self::command_to_coords(v0);
        let (x1, y1) = Self::command_to_coords(v1);
        let (x2, y2) = Self::command_to_coords(v2);

        // println!(
        //     "[GPU] Drawing shaded triangle: ({}, {}) - ({}, {}) - ({}, {}) with colors: ({}, {}, {}) - ({}, {}, {}) - ({}, {}, {})",
        //     x0, y0, x1, y1, x2, y2, r0, g0, b0, r1, g1, b1, r2, g2, b2
        // );

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

                    let mut r = (alpha * r0 as f64
                        + beta * r1 as f64
                        + gamma * r2 as f64)
                        .clamp(0.0, 255.0)
                        as u8;

                    let mut g = (alpha * g0 as f64
                        + beta * g1 as f64
                        + gamma * g2 as f64)
                        .clamp(0.0, 255.0)
                        as u8;

                    let mut b = (alpha * b0 as f64
                        + beta * b1 as f64
                        + gamma * b2 as f64)
                        .clamp(0.0, 255.0)
                        as u8;

                    let x = ((x as isize + self.x_offset) & 0x3ff) as usize; // Wrap around at 1024
                    let y = ((y as isize + self.y_offset) & 0x1ff) as usize; // Wrap around at 512

                    if x < self.min_x
                        || x > self.max_x
                        || y < self.min_y
                        || y > self.max_y
                    {
                        continue; // Skip pixels outside the clipping rectangle
                    }

                    if self.enable_dithering {
                        (r, g, b) = self.dither(x, y, r, g, b);
                    }
                    let bgr555 = Self::rgb888_to_bgr555(r, g, b);

                    self.vram[y * 1024 + x] = bgr555;
                }
            }
        }
    }

    fn textured_triangle(
        &mut self,
        modulation_color_888: u32,
        x0: usize,
        y0: usize,
        tx0: usize,
        ty0: usize,
        x1: usize,
        y1: usize,
        tx1: usize,
        ty1: usize,
        x2: usize,
        y2: usize,
        tx2: usize,
        ty2: usize,
        clut_x: usize,
        clut_y: usize,
    ) {
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
                    // We have decided that the pixel is inside the triangle
                    // We need to sample the texture at this pixel. The value in the texture is a 4-bit index
                    // for the CLUT, which is a 16-color palette.
                    let alpha = w0 / area;
                    let beta = w1 / area;
                    let gamma = w2 / area;

                    // Linearly interpolate the texture coordinates
                    let tx = (alpha * tx0 as f64
                        + beta * tx1 as f64
                        + gamma * tx2 as f64)
                        .round() as usize;
                    let ty = (alpha * ty0 as f64
                        + beta * ty1 as f64
                        + gamma * ty2 as f64)
                        .round() as usize;

                    // Calculate the texture coordinates based on the pixel position
                    let tex_x = self.tex_page_x + ((tx / 4) & 0xff);
                    let tex_y = self.tex_page_y + (ty & 0x1ff);
                    // Get the texture byte from VRAM
                    let tex_byte = self.vram[tex_y * 1024 + tex_x];
                    // Get the pixel index from the texture byte
                    let pixel_index =
                        ((tex_byte >> ((tx % 4) * 4)) & 0xf) as usize;
                    // Get the color from the CLUT
                    let clut_color =
                        self.vram[clut_y * 1024 + clut_x + pixel_index];
                    if clut_color == 0 {
                        // If the CLUT color is 0, skip this pixel (transparent)
                        continue;
                    }

                    // Blend the color with the modulation color
                    let blended_color = Self::modulate_color_555(
                        clut_color,
                        modulation_color_888,
                    );

                    // Extract RGB components from the blended color
                    let (mut r, mut g, mut b) =
                        Self::bgr555_to_rgb888(blended_color);

                    // Calculate the exact pixel coordinates using the offsets
                    let x = ((x as isize + self.x_offset) & 0x3ff) as usize; // Wrap around at 1024
                    let y = ((y as isize + self.y_offset) & 0x1ff) as usize; // Wrap around at 512

                    // Skip pixels outside the clipping rectangle
                    if x < self.min_x
                        || x > self.max_x
                        || y < self.min_y
                        || y > self.max_y
                    {
                        continue; // Skip pixels outside the clipping rectangle
                    }

                    // Apply dithering if enabled
                    if self.enable_dithering {
                        (r, g, b) = self.dither(x, y, r, g, b);
                    }

                    // Convert to BGR555 format
                    let bgr555 = Self::rgb888_to_bgr555(r, g, b);

                    // Store the pixel in VRAM
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

    fn bgr555_to_rgb888(color: u16) -> (u8, u8, u8) {
        let r = ((color & 0x1f) << 3) as u8; // 5 bits for red
        let g = (((color >> 5) & 0x1f) << 3) as u8; // 5 bits for green
        let b = (((color >> 10) & 0x1f) << 3) as u8; // 5 bits for blue

        (r, g, b)
    }

    fn command_to_coords(command: u32) -> (u16, u16) {
        let x = (command & 0x03ff) as u16;
        let y = ((command >> 16) & 0x01ff) as u16;

        (x, y)
    }

    const DITHER_MATRIX: [[i16; 4]; 4] = [
        [-4, 0, -3, 1],
        [2, -2, 3, -1],
        [-3, 1, -4, 0],
        [3, -1, 2, -2],
    ];

    fn dither(&self, x: usize, y: usize, r: u8, g: u8, b: u8) -> (u8, u8, u8) {
        let dither_value = Self::DITHER_MATRIX[y % 4][x % 4] + 4; // Shift to positive range
        let r = (r as i16 + dither_value).clamp(0, 0xff) as u8;
        let g = (g as i16 + dither_value).clamp(0, 0xff) as u8;
        let b = (b as i16 + dither_value).clamp(0, 0xff) as u8;

        (r.min(255), g.min(255), b.min(255))
    }

    fn modulate_color_555(
        orig_color_555: u16,
        modulation_color_888: u32,
    ) -> u16 {
        let (mr, mg, mb) = Self::command_to_rgb888(modulation_color_888);
        let (r, g, b) = Self::bgr555_to_rgb888(orig_color_555);

        let mr: f32 = (mr as f32) / 128.0;
        let mg: f32 = (mg as f32) / 128.0;
        let mb: f32 = (mb as f32) / 128.0;

        let r = (r as f32 * mr).clamp(0.0, 255.0) as u8;
        let g = (g as f32 * mg).clamp(0.0, 255.0) as u8;
        let b = (b as f32 * mb).clamp(0.0, 255.0) as u8;

        Self::rgb888_to_bgr555(r, g, b)
    }
}
