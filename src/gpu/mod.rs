pub struct Gpu {
    fifo: Box<[u32; 100000]>,  // FIFO for GPU commands
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

    last_cycle: u64, // Last CPU cycle count when the GPU was updated

    // GPU cycles counter used to track the dot clock. (when this value exceeds
    // the dotclock_divider, a dot is counted)
    dotclock_counter: f64,
    scanline_counter: f64, // Counter for the current scanline

    // Number of dots rendered in the current scanline
    dots: u64, // Number of dots rendered
    // Scanline counter
    scanline: u64,

    dotclock_divider: u8, // Divider for the dot clock
    interlace: bool,      // Interlaced mode flag
    is_pal: bool,         // PAL mode flag
    is_24bit: bool,       // 24-bit color mode flag

    pub is_in_vblank: bool, // Flag to indicate if the GPU is in VBlank
    pub is_in_hblank: bool, // Flag to indicate if the GPU is in HBlank

    even_odd: bool, // Flag to indicate if the GPU is in even/odd field mode
}

enum TextureBlendingMode {
    // No blending, just use the texture color. No dithering applied.
    Raw,
    // Modulate the texture color with a brightness factor (128 is normal, 255 is double brightness)
    Shaded,
    // The texture color is multiplied by the shading color.
    Modulated,
}

impl Gpu {
    pub fn new() -> Self {
        let vram = vec![0; 1024 * 512]; // Initialize VRAM with 1024x512 pixels, each pixel is 16 bits (RGB565)

        Gpu {
            fifo: Box::new([0; 100000]),
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

            last_cycle: 0,         // Initialize last cycle to 0
            dotclock_counter: 0.0, // Initialize GPU cycles to 0.0
            scanline_counter: 0.0, // Initialize scanline counter to 0.0
            dots: 0,
            scanline: 0,

            dotclock_divider: 8, // Default dot clock divider
            interlace: false,    // Default to non-interlaced mode
            is_pal: false,       // Default to NTSC mode
            is_24bit: false,     // Default to 16-bit color mode

            is_in_vblank: false, // Not in VBlank by default
            is_in_hblank: false, // Not in HBlank by default

            even_odd: false,
        }
    }

    pub fn update(
        &mut self,
        cycles: u64,
        interrupt_controller: &mut crate::interrupts::InterruptController,
    ) {
        let delta = cycles - self.last_cycle;
        self.last_cycle = cycles;

        // convert cpu cycles to GPU cycles
        let gpu_delta = delta as f64 * 1.5845;
        self.dotclock_counter += gpu_delta;
        self.scanline_counter += gpu_delta;

        if self.scanline_counter >= 2560.0 && !self.is_in_hblank {
            // Enter HBlank
            self.is_in_hblank = true;
        }

        if self.is_pal && self.scanline_counter >= 3406.0 {
            // End of scanline
            self.scanline_counter -= 3406.0;
            self.scanline += 1;

            self.dots =
                (self.scanline_counter / (self.dotclock_divider as f64)) as u64;
            self.dotclock_counter = 0.0;
            self.is_in_hblank = false; // Exit HBlank

            if self.scanline >= 240 && !self.is_in_vblank {
                // Enter VBlank
                self.is_in_vblank = true;
                interrupt_controller.trigger_irq(0); // Trigger VBlank interrupt
            }

            if !self.interlace {
                // If not interlaced, toggle even/odd field on every scanline
                self.even_odd = !self.even_odd;
            }

            if self.scanline >= 313 {
                // Reset scanline counter for PAL
                self.scanline = 0;
                self.is_in_vblank = false; // Exit VBlank

                if self.interlace {
                    // If interlaced, toggle the even/odd field on every frame
                    self.even_odd = !self.even_odd;
                }
            }
        } else if !self.is_pal && self.scanline_counter >= 3413.0 {
            // End of scanline
            self.scanline_counter -= 3413.0;
            self.scanline += 1;

            self.dots =
                (self.scanline_counter / (self.dotclock_divider as f64)) as u64;
            self.dotclock_counter = 0.0;
            self.is_in_hblank = false; // Exit HBlank

            if self.scanline >= 240 && !self.is_in_vblank {
                // Enter VBlank
                self.is_in_vblank = true;

                interrupt_controller.trigger_irq(0); // Trigger VBlank interrupt
            }

            if !self.interlace {
                // If not interlaced, toggle even/odd field on every scanline
                self.even_odd = !self.even_odd;
            }

            if self.scanline >= 262 {
                // Reset scanline counter for NTSC
                self.scanline = 0;
                self.is_in_vblank = false; // Exit VBlank

                if self.interlace {
                    // If interlaced, we need to reset the even/odd field
                    self.even_odd = !self.even_odd;
                }
            }
        }

        while self.dotclock_counter > self.dotclock_divider as f64 {
            // Reset the dot clock counter
            self.dotclock_counter -= self.dotclock_divider as f64;
            self.dots += 1;
        }
    }

    pub fn cpu_clocks_to_dotclocks(&self, clocks: u64) -> f64 {
        // Convert CPU clocks to GPU dot clocks
        (clocks as f64) * 1.5845 / self.dotclock_divider as f64
    }

    fn is_ntsc(&self) -> bool {
        !self.is_pal
    }

    pub fn get_pixel_color(&self, i: usize) -> (u8, u8, u8) {
        if self.is_24bit {
            let first = i * 3 / 2;
            if first + 1 >= self.vram.len() {
                return (0, 0, 0); // Out of bounds, return black
            }

            let h0 = self.vram[first];
            let h1 = self.vram[first + 1];

            let even = (i & 1) == 0;
            let b = if even { h0 & 0xff } else { h0 >> 8 };
            let g = if even {
                (h0 >> 8) & 0xff
            } else {
                h1 & 0xff
            };
            let r = if even {
                h1 & 0xff
            } else {
                (h1 >> 8) & 0xff
            };

            (r as u8, g as u8, b as u8)
        } else {
            let pixel = self.vram[i];

            let mut r = pixel & 0x1f; // Red channel
            let mut g = (pixel >> 5) & 0x1f; // Green channel
            let mut b = (pixel >> 10) & 0x1f; // Blue channel

            r <<= 3;
            g <<= 3;
            b <<= 3;

            (r as u8, g as u8, b as u8)
        }
    }

    pub fn read(&mut self, address: u32) -> u32 {
        if address == 0x1f80_1814 {
            let mut gpu_stat = (self.tex_page_x & 0xf) as u32;
            gpu_stat |= ((self.tex_page_y & 0x1) << 4) as u32;
            gpu_stat |= ((self.opacity_flag & 0x3) << 5) as u32;
            gpu_stat |= ((self.tex_page_depth & 0x3) << 7) as u32;
            gpu_stat |= (self.enable_dithering as u32) << 9;

            // todo bits 10, 11, 12, 14, 15, 24, 25-30
            gpu_stat |= (self.interlace as u32) << 13;

            let divider_bits = match self.dotclock_divider {
                4 => 0,
                5 => 1,
                8 => 2,
                10 => 3,
                7 => 4,
                _ => unreachable!(),
            };

            if divider_bits == 4 {
                gpu_stat |= 1 << 16;
            } else {
                gpu_stat |= divider_bits << 17;
            }

            gpu_stat |= (self.interlace as u32) << 19;
            gpu_stat |= (self.is_pal as u32) << 20;
            gpu_stat |= (self.is_24bit as u32) << 21;
            gpu_stat |= (self.interlace as u32) << 22;
            gpu_stat |= (self.display_enable as u32) << 23;

            let even_odd = self.even_odd && !self.is_in_vblank;
            gpu_stat |= (even_odd as u32) << 31; // Set the even/odd field bit

            // TMP force bits 26-28 (readyness)
            gpu_stat |= 0x7 << 26; // Force bits 26-28 to 111

            return gpu_stat;
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
                    0x08 => {
                        let dividers = [10, 8, 5, 4];
                        if value & (1 << 6) != 0 {
                            self.dotclock_divider = 7;
                        } else {
                            let divider_index = (value >> 6) & 3;
                            self.dotclock_divider =
                                dividers[divider_index as usize];
                        }

                        self.interlace = (value & 0x14) == 0x14;
                        self.is_pal = value & 0x80 != 0;
                        self.is_24bit = value & 0x10 != 0;
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

        let command = first >> 24;
        let primary = command >> 5;

        return match primary {
            0 => {
                if command == 2 {
                    3
                } else {
                    1
                }
            }
            1 | 2 | 3 => {
                // Drawing commands
                let primitive = primary; // 1 = polygon, 2 = line, 3 = rectangle

                match primitive {
                    1 => {
                        let gourad = (command & 0x10) != 0; // Gouraud shading flag
                        let quad = (command & 0x8) != 0; // Quad flag (or triangle if 0)
                        let texture_mapping = (command & 0x4) != 0; // Texture mapping flag

                        let words_per_vertex = {
                            if gourad {
                                if texture_mapping {
                                    3
                                } else {
                                    2
                                }
                            } else {
                                if texture_mapping {
                                    2
                                } else {
                                    1
                                }
                            }
                        };

                        let vertices = if quad {
                            4
                        } else {
                            3
                        };

                        vertices * words_per_vertex + if !gourad {
                            1 // Color word for flat shading
                        } else {
                            0 // No color word for Gouraud shading
                        }
                    }
                    2 => {
                        // bits 0 and 2 are unused
                        let gourad = (command & 0x10) != 0; // Gouraud shading flag
                        let polyline = (command & 0x8) != 0; // Quad flag (or triangle if 0)

                        if polyline {
                            if self.fifo[self.fifo_index - 1] & 0x50005000 == 0x50005000 {
                                self.fifo_index
                            } else {
                                16
                            }
                        } else {
                            if gourad {
                                4
                            } else {
                                3
                            }
                        }
                    }
                    3 => {
                        // Rectangle command
                        let size = (command >> 3) & 3; // 0 = variable, 1 = 1x1, 2 = 8x8, 3 = 16x16
                        let texture_mapping = (command & 0x4) != 0; // Texture mapping flag

                        if texture_mapping {
                            if size == 0 {
                                4
                            } else {
                                3
                            }
                        } else {
                            if size == 0 {
                                3
                            } else {
                                2
                            }
                        }
                    }
                    _ => {
                        unreachable!()
                    }
                }
            }
            4 => {
                // VRAM-to-VRAM transfer
                4
            }
            5 => {
                // CPU-to-VRAM transfer
                if self.fifo.len() < 3 {
                    return 1000; // Not enough data in FIFO
                }

                let ys = self.fifo[2] >> 16;
                let xs = self.fifo[2] & 0xffff;

                let transfer_words = (ys * xs + 1) / 2;
                3 + transfer_words as usize // 3 for the command and 1 for each pair of pixels
            }
            6 => {
                // VRAM-to-CPU transfer
                3
            }
            7 => {
                // Drawing settings
                1
            }
            _ => unreachable!(),
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
            0x01 => {
                // clear fifo
                self.fifo_index = 0;
            }
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
                        self.vram[idx] = c;
                    }
                }
            }
            0xa0 => {
                // println!(
                //     "[GPU] Drawing pixels to VRAM: {:?}",
                //     &self.fifo[0..=2]
                // );

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
                // println!(
                //     "[GPU] Reading pixels from VRAM: {:?}",
                //     &self.fifo[0..self.fifo_index]
                // );
                self.reading_x = (self.fifo[1] & 0x3ff) as usize; // X position
                self.reading_y = ((self.fifo[1] >> 16) & 0x1ff) as usize; // Y position

                let size_x = (self.fifo[2] & 0x3ff) as usize; // Width
                let size_y = ((self.fifo[2] >> 16) & 0x1ff) as usize; // Height

                let total_halfwords = size_x * size_y;
                let total_words = (total_halfwords + 1) / 2; // Round up if odd

                self.is_reading = total_words;
            }
            0x20 | 0x22 | 0x28 | 0x2a => {
                // Monochrome 3/4 point polygon. Opaque/semi-transparent.
                let semi_transparent = command & 2 != 0;
                let quad = command & 8 != 0;

                let v1 = VertexData::from_command(self.fifo[1], self.fifo[0]);
                let v2 = VertexData::from_command(self.fifo[2], self.fifo[0]);
                let v3 = VertexData::from_command(self.fifo[3], self.fifo[0]);

                self.triangle(v1, v2, v3, semi_transparent, true);

                if quad {
                    let v4 = VertexData::from_command(self.fifo[4], self.fifo[0]);

                    self.triangle(v2, v3, v4, semi_transparent, true);
                }
            }
            0x24..=0x27 | 0x2c..=0x2f => {
                // Textured 3/4 point polygon. Opaque/semi-transparent. Texture blending or raw
                let quad = command & 8 != 0;
                let semi_transparent = command & 2 != 0;
                let blending = if command & 4 != 0 { 
                    TextureBlendingMode::Modulated
                } else {
                    TextureBlendingMode::Raw
                };

                let clut = self.fifo[2] >> 16;
                let tex_page = self.fifo[4] >> 16;

                self.update_texture_page(tex_page);

                let v1 = TexturedVertexData::from_command(self.fifo[1], self.fifo[0], self.fifo[2]);
                let v2 = TexturedVertexData::from_command(self.fifo[3], self.fifo[0], self.fifo[4]);
                let v3 = TexturedVertexData::from_command(self.fifo[5], self.fifo[0], self.fifo[6]);

                // self.textured_triangle(v1, v2, v3, semi_transparent, blending);

                if quad {
                    let v4 = TexturedVertexData::from_command(self.fifo[7], self.fifo[0], self.fifo[8]);

                    // self.textured_triangle(v1, v2, v3, semi_transparent, blending);
                }
            }
            0x30 | 0x32 | 0x38 | 0x3a => {
                // Shaded 3/4 point polygon. Opaque/semi-transparent.
                let quad = command & 8 != 0;
                let semi_transparent = command & 2 != 0;

                let v1 = VertexData::from_command(self.fifo[1], self.fifo[0]);
                let v2 = VertexData::from_command(self.fifo[3], self.fifo[2]);
                let v3 = VertexData::from_command(self.fifo[5], self.fifo[4]);

                self.triangle(v1, v2, v3, semi_transparent, false);

                if quad {
                    let v4 = VertexData::from_command(self.fifo[7], self.fifo[6]);

                    self.triangle(v2, v3, v4, semi_transparent, false);
                }
            }
            0x34 | 0x36 | 0x3c | 0x3e => {
                // Shaded, textured 3/4 point polygon. Opaque/semi-transparent.
                let quad = command & 8 != 0;
                let semi_transparent = command & 2 != 0;

                let clut = self.fifo[2] >> 16; // CLUT address
                let tex_page = self.fifo[5] >> 16; // Texture page address

                self.update_texture_page(tex_page);

                let v1 = TexturedVertexData::from_command(self.fifo[1], self.fifo[0], self.fifo[2]);
                let v2 = TexturedVertexData::from_command(self.fifo[4], self.fifo[3], self.fifo[5]);
                let v3 = TexturedVertexData::from_command(self.fifo[7], self.fifo[6], self.fifo[8]);

                // self.textured_triangle(v1, v2, v3, semi_transparent, TextureBlendingMode::Shaded);

                if quad {
                    let v4 = TexturedVertexData::from_command(self.fifo[10], self.fifo[9], self.fifo[11]);

                    // self.textured_triangle(v2, v3, v4, semi_transparent, TextureBlendingMode::Shaded);
                }
            }
            // 0x2c => {
            //     // Similar 0x64. However:
            //     // - takes 4 vertices instead of 1 + (w, h)
            //     // - takes the texpage inline, instead of from previous E1 command (actually, this updates the state of the E1 command)
            //     // - uses the CLUT just like 0x64, however
            //     // - each vertex has it's own texture coordinates
            //     //
            //     // And, crucially, this is rendered as two triangles, not a rectangle.

            //     // Textured Rectangle, variable size, opaque, texture-blending
            //     let modulation_color = self.fifo[0] & 0xffffff; // Color
            //     let x0 = (self.fifo[1] & 0x3ff) as usize; // X position
            //     let y0 = ((self.fifo[1] >> 16) & 0x1ff) as usize; // Y position

            //     let x1 = (self.fifo[3] & 0x3ff) as usize; // X position
            //     let y1 = ((self.fifo[3] >> 16) & 0x1ff) as usize; // Y position

            //     let x2 = (self.fifo[5] & 0x3ff) as usize; // X position
            //     let y2 = ((self.fifo[5] >> 16) & 0x1ff) as usize; // Y position

            //     let x3 = (self.fifo[7] & 0x3ff) as usize; // X position
            //     let y3 = ((self.fifo[7] >> 16) & 0x1ff) as usize; // Y position

            //     let clut = (self.fifo[2] >> 16) as usize; // CLUT address
            //     let clut_x = (clut & 0x3f) * 16;
            //     let clut_y = (clut >> 6) & 0x1ff;

            //     let texpage = (self.fifo[4] >> 16) as usize; // Texture page address
            //     self.tex_page_x = (texpage & 0xf) * 64; // X position
            //     self.tex_page_y = ((texpage >> 4) & 0x1) * 256; // Y position

            //     let tx0 = (self.fifo[2] & 0xff) as usize; // Texture X position
            //     let ty0 = ((self.fifo[2] >> 8) & 0xff) as usize; // Texture Y position

            //     let tx1 = (self.fifo[4] & 0xff) as usize; // Texture X position
            //     let ty1 = ((self.fifo[4] >> 8) & 0xff) as usize; // Texture Y position

            //     let tx2 = (self.fifo[6] & 0xff) as usize; // Texture X position
            //     let ty2 = ((self.fifo[6] >> 8) & 0xff) as usize; // Texture Y position

            //     let tx3 = (self.fifo[8] & 0xff) as usize; // Texture X position
            //     let ty3 = ((self.fifo[8] >> 8) & 0xff) as usize; // Texture Y position

            //     self.textured_triangle(
            //         modulation_color,
            //         x0,
            //         y0,
            //         tx0,
            //         ty0,
            //         x1,
            //         y1,
            //         tx1,
            //         ty1,
            //         x2,
            //         y2,
            //         tx2,
            //         ty2,
            //         clut_x,
            //         clut_y,
            //     );

            //     self.textured_triangle(
            //         modulation_color,
            //         x1,
            //         y1,
            //         tx1,
            //         ty1,
            //         x2,
            //         y2,
            //         tx2,
            //         ty2,
            //         x3,
            //         y3,
            //         tx3,
            //         ty3,
            //         clut_x,
            //         clut_y,
            //     );
            // }
            // 0x30 | 0x32 | 0x38 | 0x3a => {
            //     let opaque = command & 2 == 0;

            //     // Shaded 3/4 point polygon. Opaque/semi-transparent.
            //     let c0 = self.fifo[0] & 0xffffff;
            //     let v0 = self.fifo[1];
            //     let c1 = self.fifo[2] & 0xffffff;
            //     let v1 = self.fifo[3];
            //     let c2 = self.fifo[4] & 0xffffff;
            //     let v2 = self.fifo[5];

            //     self.shaded_triangle(c0, v0, c1, v1, c2, v2, opaque);

            //     if command & 8 == 0x8 {
            //         let c3 = self.fifo[6] & 0xffffff;
            //         let v3 = self.fifo[7];

            //         self.shaded_triangle(c1, v1, c2, v2, c3, v3, opaque);
            //     }
            // }
            // 0x34 | 0x3c => {
            //     // Shaded textured four point polygon, variable size, opaque, texture-blending

            //     let clut = self.fifo[2] >> 16; // CLUT address
            //     let clut_x = (clut & 0x3f) * 16;
            //     let clut_y = (clut >> 6) & 0x1ff;

            //     let page = self.fifo[5] >> 16;
            //     self.tex_page_x = ((page & 0xf) * 64) as usize; // Texture page X position
            //     self.tex_page_y = (((page >> 4) & 0x1) * 256) as usize; // Texture page Y position

            //     let tex_x = (self.fifo[2] & 0xff) as usize; // Texture X position
            //     let tex_y = ((self.fifo[2] >> 8) & 0xff) as usize; // Texture Y position

            //     let x0 = (self.fifo[1] & 0x3ff) as usize; // X position
            //     let y0 = ((self.fifo[1] >> 16) & 0x1ff) as usize; // Y position
            //     let c0 = self.fifo[0] & 0xffffff; // Color

            //     let x1 = (self.fifo[4] & 0x3ff) as usize; // X position
            //     let y1 = ((self.fifo[4] >> 16) & 0x1ff) as usize; // Y position
            //     let c1 = self.fifo[3] & 0xffffff; // Color

            //     let x2 = (self.fifo[7] & 0x3ff) as usize; // X position
            //     let y2 = ((self.fifo[7] >> 16) & 0x1ff) as usize; // Y position
            //     let c2 = self.fifo[6] & 0xffffff; // Color

            //     self.shaded_triangle(
            //         c0,
            //         self.fifo[1],
            //         c1,
            //         self.fifo[4],
            //         c2,
            //         self.fifo[7],
            //         true
            //     );

            //     if command == 0x3c {
            //         let x3 = (self.fifo[10] & 0x3ff) as usize; // X position
            //         let y3 = ((self.fifo[10] >> 16) & 0x1ff) as usize; // Y position
            //         let c3 = self.fifo[9] & 0xffffff; // Color

            //         self.shaded_triangle(
            //             c1,
            //             self.fifo[4],
            //             c2,
            //             self.fifo[7],
            //             c3,
            //             self.fifo[10],
            //             true
            //         );
            //     }

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

                // for y in 0..h {
                //     for x in 0..w {
                //         let tex_x_pos =
                //             self.tex_page_x + ((tex_x + x / 4) % 256); // Wrap around at 256
                //         let tex_y_pos = self.tex_page_y + ((tex_y + y) % 512); // Wrap around at 512

                //         // Get the texture byte
                //         let tex_byte = self.vram[tex_y_pos * 1024 + tex_x_pos];

                //         // Get the pixel index from the texture byte
                //         let pixel_index =
                //             ((tex_byte >> ((x % 4) * 4)) & 0xf) as usize;

                //         // println!(
                //         //     "[GPU] Sampling texture at ({}, {}) for pixel ({}, {}). Idx: {}",
                //         //     tex_x_pos, tex_y_pos, x, y, pixel_index
                //         // );

                //         // Get the color from the CLUT
                //         let clut_color = self.vram[(clut_y * 1024
                //             + clut_x
                //             + pixel_index as u32)
                //             as usize];

                //         if clut_color == 0 {
                //             // If the CLUT color is 0, skip this pixel (transparent)
                //             continue;
                //         }

                //         // println!(
                //         //     "[GPU] CLUT color for index {}: {:#x}",
                //         //     pixel_index, clut_color
                //         // );

                //         // Blend the color with the command color (TODO)
                //         let blended_color = Self::modulate_color_555(
                //             clut_color,
                //             modulation_color,
                //         );

                //         let idx =
                //             ((y0 + y) & 0x1ff) * 1024 + ((x0 + x) & 0x3ff);
                //         self.vram[idx] = blended_color as u16;
                //     }
                // }
            // }
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
            0x64 | 0x7c => {
                // Textured Rectangle, variable size, opaque, texture-blending
                let modulation_color = self.fifo[0] & 0xffffff; // Color
                let x0 = (self.fifo[1] & 0x3ff) as usize; // X position
                let y0 = ((self.fifo[1] >> 16) & 0x1ff) as usize; // Y position

                let w = if command == 0x64 { (self.fifo[3] & 0x3ff) as usize } else { 16 };
                let h = if command == 0x64 { ((self.fifo[3] >> 16) & 0x1ff) as usize } else { 16 };

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

    fn update_texture_page(&mut self, page: u32) {
        self.tex_page_x = ((page & 0xf) * 64) as usize; // Texture page X position
        self.tex_page_y = (((page >> 4) & 0x1) * 256) as usize; // Texture page Y position
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

    fn triangle(
        &mut self,
        v1: VertexData,
        v2: VertexData,
        v3: VertexData,
        semi_transparent: bool,
        force_dither_off: bool,
    ) {
        // This function should implement the logic to draw a shaded triangle
        // using the provided vertex colors and coordinates.

        // Convert colors to RGB888 format
        let (r0, g0, b0) = v1.color.explode();
        let (r1, g1, b1) = v2.color.explode();
        let (r2, g2, b2) = v3.color.explode();

        // Convert vertex coordinates to (x, y) pairs
        let (x0, y0) = v1.vertex.explode();
        let (x1, y1) = v2.vertex.explode();
        let (x2, y2) = v3.vertex.explode();

        // TODOs:
        // - Handle semi-transparency
        // - Do not render if any two vertices are 1024 or more horizontal pixels apart or 512 or more vertical pixels apart

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

                    if self.enable_dithering && !force_dither_off {
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

#[derive(Debug, Clone, Copy)]
struct VertexData {
    vertex: Vertex,
    color: Color,
}

impl VertexData {
    fn new(vertex: Vertex, color: Color) -> Self {
        Self { vertex, color }
    }

    fn from_command(vertex_word: u32, color_word: u32) -> Self {
        let vertex = Vertex::from_command(vertex_word);
        let color = Color::from_command(color_word);
        Self { vertex, color }
    }
}

#[derive(Debug, Clone, Copy)]
struct TexturedVertexData {
    vertex: Vertex,
    color: Color,
    texture_coords: TextureCoords,
}

impl TexturedVertexData {
    fn new(vertex: Vertex, color: Color, texture_coords: TextureCoords) -> Self {
        Self { vertex, color, texture_coords }
    }

    fn from_command(vertex_word: u32, color_word: u32, texture_word: u32) -> Self {
        let vertex = Vertex::from_command(vertex_word);
        let color = Color::from_command(color_word);
        let texture_coords = TextureCoords::from_command(texture_word);
        Self { vertex, color, texture_coords }
    }
}

#[derive(Debug, Clone, Copy)]
struct Vertex {
    x: isize,
    y: isize,
}

impl Vertex {
    fn new(x: isize, y: isize) -> Self {
        Self { x, y }
    }

    fn explode(self) -> (isize, isize) {
        (self.x, self.y)
    }

    fn from_command(command: u32) -> Self {
        // The command encodes the vertex position as two 11-bit signed values.
        // Each occupies 16 bits in the command, with the top 5 bits ignored.
 
        // Bit 11 is the sign and must be extended to 16 bits to get the correct value.

        let x = (command & 0x7ff) as i16; // X position (11 bits)
        let y = ((command >> 16) & 0x7ff) as i16; // Y position (11 bits)
        let x = (x << 5) >> 5; // Sign-extend to 16 bits
        let y = (y << 5) >> 5; // Sign-extend to 16

        Self { x: x as isize, y: y as isize }
    }
}

#[derive(Debug, Clone, Copy)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    fn explode(self) -> (u8, u8, u8) {
        (self.r, self.g, self.b)
    }

    fn from_command(rgb: u32) -> Self {
        let r = (rgb & 0xff) as u8;
        let g = ((rgb >> 8) & 0xff) as u8;
        let b = ((rgb >> 16) & 0xff) as u8;

        Self::new(r, g, b)
    }

    fn to_bgr555(&self) -> u16 {
        let r = (self.r >> 3) & 0x1f; // 5 bits for red
        let g = (self.g >> 3) & 0x1f; // 5 bits for green
        let b = (self.b >> 3) & 0x1f; // 5 bits for blue

        (r as u16) | ((g as u16) << 5) | ((b as u16) << 10)
    }

    fn as_modulation(&self) -> (f32, f32, f32) {
        (
            (self.r as f32) / 128.0,
            (self.g as f32) / 128.0,
            (self.b as f32) / 128.0,
        )
    }

    fn modulate(&self, other: &Color) -> Color {
        let (mr, mg, mb) = other.as_modulation();
        let r = (self.r as f32 * mr).min(255.0) as u8;
        let g = (self.g as f32 * mg).min(255.0) as u8;
        let b = (self.b as f32 * mb).min(255.0) as u8;

        Color::new(r, g, b)
    }

    fn multiply(&self, other: &Color) -> Color {
        let r = (self.r as f64 * other.r as f64 / 255.0) as u8;
        let g = (self.g as f64 * other.g as f64 / 255.0) as u8;
        let b = (self.b as f64 * other.b as f64 / 255.0) as u8;

        Color::new(r, g, b)
    }
}

#[derive(Debug, Clone, Copy)]
struct Sizing {
    width: usize,
    height: usize,
}

impl Sizing {
    fn new(width: usize, height: usize) -> Self {
        Self { width, height }
    }

    fn from_command(command: u32) -> Self {
        let width = (command & 0x3ff) as usize; // Width (10 bits)
        let height = ((command >> 16) & 0x1ff) as usize; // Height (9 bits)

        Self::new(width, height)
    }
}

#[derive(Debug, Clone, Copy)]
struct TextureCoords {
    x: usize,
    y: usize,
}

impl TextureCoords {
    fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }

    fn from_command(command: u32) -> Self {
        let x = (command & 0xff) as usize; // X position (8 bits)
        let y = ((command >> 8) & 0xff) as usize; // Y position (8 bits)

        Self::new(x, y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_from_command() {
        let command = 0x0400_03ff;
        let vertex = Vertex::from_command(command);
        assert_eq!(vertex.x, 1023);
        assert_eq!(vertex.y, -1024);
    }
}