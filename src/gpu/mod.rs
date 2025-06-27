pub struct Gpu {
    fifo: Box<[u32; 100000]>, // FIFO for GPU commands
    fifo_index: usize,        // Current index in the FIFO
    pub vram: Vec<u16>,       // Video RAM for storing pixel data
    is_reading: usize,
    reading_x: usize, // X coordinate for reading pixels
    reading_y: usize, // Y coordinate for reading pixels
    reading_size_x: usize, // Width of the area being read
    reading_size_y: usize, // Height of the area being read
    reading_cur_x: usize, // Current X coordinate in the reading process
    reading_cur_y: usize, // Current Y coordinate in the reading process

    x_offset: isize,
    y_offset: isize,
    min_x: usize,
    min_y: usize,
    max_x: usize,
    max_y: usize,

    // Coordinates in VRAM where the data to send to the screen starts
    display_x_start: usize,
    display_y_start: usize,

    // Range of visible pixels on the screen expressed in dotclocks and scanlines
    display_x1: usize,
    display_y1: usize,
    display_x2: usize,
    display_y2: usize,

    display_enable: bool, // Flag to enable/disable display
    // Direction of DMA transfer (0 off, 1 regular FIFO, 2 sending from CPU, 3 sending to CPU)
    dma_direction: u32,

    enable_dithering: bool, // Flag to enable dithering

    tex_page_x: usize,
    tex_page_y: usize,
    opacity_flag: usize,
    tex_page_depth: usize, // Color depth of the texture page

    last_cycle: usize, // Last CPU cycle count when the GPU was updated

    // GPU cycles counter used to track the dot clock. (when this value exceeds
    // the dotclock_divider, a dot is counted)
    dotclock_counter: f64,
    scanline_counter: f64, // Counter for the current scanline

    // Number of dots rendered in the current scanline
    dots: u64, // Number of dots rendered
    // Scanline counter
    scanline: usize,

    dotclock_divider: u8, // Divider for the dot clock
    interlace: bool,      // Interlaced mode flag
    is_pal: bool,         // PAL mode flag
    is_24bit: bool,       // 24-bit color mode flag

    pub is_in_vblank: bool, // Flag to indicate if the GPU is in VBlank
    pub is_in_hblank: bool, // Flag to indicate if the GPU is in HBlank

    even_odd: bool, // Flag to indicate if the GPU is in even/odd field mode
}

#[derive(Debug, Clone, Copy, PartialEq)]
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
            reading_size_x: 0,
            reading_size_y: 0,
            reading_cur_x: 0,
            reading_cur_y: 0,

            x_offset: 0,
            y_offset: 0,

            min_x: 0,
            min_y: 0,
            max_x: 1023,
            max_y: 511,

            display_x_start: 0,
            display_y_start: 0,

            display_x1: 512,
            display_y1: 16,
            display_x2: 3072,
            display_y2: 256,

            display_enable: false, // Display is off by default
            dma_direction: 0,      // DMA is off by default
            enable_dithering: false, // Dithering is disabled by default

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

    pub fn effective_resolution(&self) -> (usize, usize) {
        let x = (self.display_x2 - self.display_x1)
            / self.dotclock_divider as usize;

        if self.is_pal {
            (x, 486)
        } else {
            (x, 486)
        }
    }

    pub fn get_screen_pixel(&self, x: usize, y: usize) -> (u8, u8, u8) {
        let x = (self.display_x_start + x) & 0x3ff; // Wrap around at 1024

        let y = if self.interlace {
            y
        } else {
            // Line doubling for non-interlaced mode
            y / 2
        };

        let y = (self.display_y_start + y) & 0x1ff; // Wrap around at 512

        if self.is_24bit {
            // each pixel occupies 3 bytes. However we treat VRAM as u16 array, so the math for X is a bit different
            let pixel_index = y * 1024 + (x * 3 / 2);
            let h0 = self.vram[pixel_index];
            let h1 = self.vram[pixel_index + 1];

            let even = (x & 1) == 0;
            let r = if even { h0 & 0xff } else { h0 >> 8 };
            let g = if even { (h0 >> 8) & 0xff } else { h1 & 0xff };
            let b = if even { h1 & 0xff } else { (h1 >> 8) & 0xff };
            (r as u8, g as u8, b as u8)
        } else {
            let pixel_index = y * 1024 + x;
            let bgr555 = self.vram[pixel_index];
            Color::from_bgr555(bgr555).to_rgb888()
        }
    }

    pub fn get_raw_vram_color(&self, x: usize, y: usize) -> (u8, u8, u8) {
        let pixel_index = y * 1024 + x;
        let bgr555 = self.vram[pixel_index];
        Color::from_bgr555(bgr555).to_rgb888()
    }

    pub fn update(
        &mut self,
        cycles: usize,
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

            if self.scanline == self.display_y2 {
                // println!("[GPU] ENTERING VBLANK {} {} {}", self.display_y1, self.display_y2, self.scanline);
                self.is_in_vblank = true;
                interrupt_controller.trigger_irq(0); // Trigger VBlank interrupt
            } else if self.scanline == self.display_y1 {
                // println!("[GPU] EXITING VBLANK. Avoid drawing?");
                self.is_in_vblank = false;
            }

            if !self.interlace {
                // If not interlaced, toggle even/odd field on every scanline
                self.even_odd = !self.even_odd;
            }

            if self.scanline >= 313 {
                // Reset scanline counter for PAL
                self.scanline = 0;

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

            if self.scanline == self.display_y2 {
                // println!("[GPU] ENTERING VBLANK {} {} {}", self.display_y1, self.display_y2, self.scanline);
                self.is_in_vblank = true;

                interrupt_controller.trigger_irq(0); // Trigger VBlank interrupt
            } else if self.scanline == self.display_y1 {
                // println!("[GPU] EXITING VBLANK. Avoid drawing?");
                self.is_in_vblank = false;
            }

            if !self.interlace {
                // If not interlaced, toggle even/odd field on every scanline
                self.even_odd = !self.even_odd;
            }

            if self.scanline >= 262 {
                // Reset scanline counter for NTSC
                self.scanline = 0;

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

    pub fn cpu_clocks_to_dotclocks(&self, clocks: usize) -> f64 {
        // Convert CPU clocks to GPU dot clocks
        (clocks as f64) * 1.5845 / self.dotclock_divider as f64
    }

    pub fn read(&mut self, address: u32) -> u32 {
        if address == 0x1f80_1814 {
            let mut gpu_stat = (self.tex_page_x & 0xf) as u32;
            gpu_stat |= ((self.tex_page_y & 0x1) << 4) as u32;
            gpu_stat |= ((self.opacity_flag & 0x3) << 5) as u32;
            gpu_stat |= ((self.tex_page_depth & 0x3) << 7) as u32;
            gpu_stat |= (self.enable_dithering as u32) << 9;

            // todo bits 10, 11, 12, 14, 15, 24, 25-28
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

            gpu_stat |= match self.dma_direction {
                1 => 1, // Regular FIFO
                2 => self.is_reading as u32, // CPU to VRAM
                3 => 1, // TODO VRAM to CPU
                _ => 0,
            } << 25;

            // TMP force bits 26-28 (readyness)
            gpu_stat |= 0x7 << 26; // Force bits 26-28 to 111

            gpu_stat |= (self.dma_direction) << 29;

            let even_odd = self.even_odd && !self.is_in_vblank;
            gpu_stat |= (even_odd as u32) << 31; // Set the even/odd field bit

            return gpu_stat;
        }

        // println!("[GPU] Read operation at address {:#x}", address);
        if self.is_reading > 0 {
            let x = (self.reading_x + self.reading_cur_x) % 1024;
            let y = (self.reading_y + self.reading_cur_y) % 512;

            let px0 = self.vram[y * 1024 + x];
            self.reading_cur_x += 1;
            if self.reading_cur_x == self.reading_size_x {
                self.reading_cur_x = 0;
                self.reading_cur_y += 1;
            }

            let x = (self.reading_x + self.reading_cur_x) % 1024;
            let y = (self.reading_y + self.reading_cur_y) % 512;

            let px1 = self.vram[y * 1024 + x];
            self.reading_cur_x += 1;
            if self.reading_cur_x == self.reading_size_x {
                self.reading_cur_x = 0;
                self.reading_cur_y += 1;
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

                        self.display_x_start = 0;
                        self.display_y_start = 0;

                        self.display_x1 = 512;
                        self.display_y1 = 16;
                        self.display_x2 = 3072;
                        self.display_y2 = 256;
                    }
                    0x03 => {
                        self.display_enable = (value & 0x1) == 0;
                    }
                    0x04 => {
                        self.dma_direction = cmd & 3;
                    }
                    0x05 => {
                        self.display_x_start = (value & 0x3ff) as usize; // X start position
                        self.display_y_start = ((value >> 10) & 0x1ff) as usize; // Y start position
                    }
                    0x06 => {
                        self.display_x1 = (value & 0xfff) as usize;
                        self.display_x2 = ((value >> 12) & 0xfff) as usize;
                    }
                    0x07 => {
                        self.display_y1 = (value & 0x3ff) as usize;
                        self.display_y2 = ((value >> 10) & 0x3ff) as usize;

                        println!(
                            "[GPU] Display area set to: ({}, {})",
                            self.display_y1, self.display_y2
                        );
                    }
                    0x08 => {
                        // println!(
                        //     "[GPU] GP1(08h) - Display mode: {:x}",
                        //     value & 0xffffff
                        // );

                        let dividers = [10, 8, 5, 4];
                        if value & (1 << 6) != 0 {
                            self.dotclock_divider = 7;
                        } else {
                            let divider_index = value & 3;
                            self.dotclock_divider =
                                dividers[divider_index as usize];
                        }

                        self.interlace = (value & 0x24) == 0x24;
                        self.is_pal = value & (1 << 3) != 0;
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
                                if texture_mapping { 3 } else { 2 }
                            } else {
                                if texture_mapping { 2 } else { 1 }
                            }
                        };

                        let vertices = if quad { 4 } else { 3 };

                        vertices * words_per_vertex
                            + if !gourad {
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
                            if self.fifo[self.fifo_index - 1] & 0x50005000
                                == 0x50005000
                            {
                                self.fifo_index
                            } else {
                                10000
                            }
                        } else {
                            if gourad { 4 } else { 3 }
                        }
                    }
                    3 => {
                        // Rectangle command
                        let size = (command >> 3) & 3; // 0 = variable, 1 = 1x1, 2 = 8x8, 3 = 16x16
                        let texture_mapping = (command & 0x4) != 0; // Texture mapping flag

                        if texture_mapping {
                            if size == 0 { 4 } else { 3 }
                        } else {
                            if size == 0 { 3 } else { 2 }
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
        };
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
            0x1a => {
                // ignored
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
                let mut dst_idx = dest_y * 1024 + dest_x;

                for _ in 0..ys {
                    for xoffs in 0..xs {
                        self.vram[dst_idx + xoffs] = data[src_idx];
                        src_idx += 1;
                    }
                    dst_idx += 1024;
                }
            }
            0x80 => {
                // Copy a rectangle from VRAM to another rectangle in VRAM
                let src = Vertex::from_command(self.fifo[1]);
                let dst = Vertex::from_command(self.fifo[2]);
                let width = (self.fifo[3] & 0x3ff) as usize; // Width
                let height = ((self.fifo[3] >> 16) & 0x1ff) as usize; // Height

                for y in 0..height {
                    for x in 0..width {
                        let src_x = (src.x as usize + x) & 0x3ff; // Wrap around at 1024
                        let src_y = (src.y as usize + y) & 0x1ff; // Wrap around at 512
                        let dst_x = (dst.x as usize + x) & 0x3ff; // Wrap around at 1024
                        let dst_y = (dst.y as usize + y) & 0x1ff; // Wrap around at 512

                        let src_idx = src_y * 1024 + src_x;
                        let dst_idx = dst_y * 1024 + dst_x;

                        self.vram[dst_idx] = self.vram[src_idx];
                    }
                }
            }
            0xc0 => {
                self.reading_x = (self.fifo[1] & 0x3ff) as usize; // X position
                self.reading_y = ((self.fifo[1] >> 16) & 0x1ff) as usize; // Y position

                self.reading_cur_x = 0;
                self.reading_cur_y = 0;

                let size_x = (self.fifo[2] & 0x3ff) as usize; // Width
                let size_y = ((self.fifo[2] >> 16) & 0x1ff) as usize; // Height

                self.reading_size_x = size_x;
                self.reading_size_y = size_y;

                let total_halfwords = size_x * size_y;
                let total_words = (total_halfwords + 1) / 2; // Round up if odd

                self.is_reading = total_words;
            }
            0x20 | 0x22 | 0x28 | 0x2a => {
                // Monochrome 3/4 point polygon. Opaque/semi-transparent.
                let semi_transparent = command & 2 != 0;
                let quad = command & 8 != 0;

                let v1 = VertexData::from_command(self.fifo[0], self.fifo[1]);
                let v2 = VertexData::from_command(self.fifo[0], self.fifo[2]);
                let v3 = VertexData::from_command(self.fifo[0], self.fifo[3]);

                self.triangle(v1, v2, v3, semi_transparent, true);

                if quad {
                    let v4 =
                        VertexData::from_command(self.fifo[0], self.fifo[4]);

                    self.triangle(v2, v3, v4, semi_transparent, true);
                }
            }
            0x24..=0x27 | 0x2c..=0x2f => {
                // Textured 3/4 point polygon. Opaque/semi-transparent. Texture blending or raw
                let quad = command & 8 != 0;
                let semi_transparent = command & 2 != 0;
                let blending = if command & 1 != 0 {
                    TextureBlendingMode::Raw
                } else {
                    TextureBlendingMode::Modulated
                };

                let clut = self.fifo[2] >> 16;
                let tex_page = self.fifo[4] >> 16;

                self.update_texture_page(tex_page);

                let v1 = TexturedVertexData::from_command(
                    self.fifo[0],
                    self.fifo[1],
                    self.fifo[2],
                );
                let v2 = TexturedVertexData::from_command(
                    self.fifo[0],
                    self.fifo[3],
                    self.fifo[4],
                );
                let v3 = TexturedVertexData::from_command(
                    self.fifo[0],
                    self.fifo[5],
                    self.fifo[6],
                );

                self.textured_triangle(
                    v1,
                    v2,
                    v3,
                    clut as u16,
                    semi_transparent,
                    blending,
                );

                if quad {
                    let v4 = TexturedVertexData::from_command(
                        self.fifo[0],
                        self.fifo[7],
                        self.fifo[8],
                    );

                    self.textured_triangle(
                        v2,
                        v3,
                        v4,
                        clut as u16,
                        semi_transparent,
                        blending,
                    );
                }
            }
            0x30 | 0x32 | 0x38 | 0x3a => {
                // Shaded 3/4 point polygon. Opaque/semi-transparent.
                let quad = command & 8 != 0;
                let semi_transparent = command & 2 != 0;

                let v1 = VertexData::from_command(self.fifo[0], self.fifo[1]);
                let v2 = VertexData::from_command(self.fifo[2], self.fifo[3]);
                let v3 = VertexData::from_command(self.fifo[4], self.fifo[5]);

                self.triangle(v1, v2, v3, semi_transparent, false);

                if quad {
                    let v4 =
                        VertexData::from_command(self.fifo[6], self.fifo[7]);

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

                let v1 = TexturedVertexData::from_command(
                    self.fifo[0],
                    self.fifo[1],
                    self.fifo[2],
                );
                let v2 = TexturedVertexData::from_command(
                    self.fifo[3],
                    self.fifo[4],
                    self.fifo[5],
                );
                let v3 = TexturedVertexData::from_command(
                    self.fifo[6],
                    self.fifo[7],
                    self.fifo[8],
                );

                self.textured_triangle(
                    v1,
                    v2,
                    v3,
                    clut as u16,
                    semi_transparent,
                    TextureBlendingMode::Shaded,
                );

                if quad {
                    let v4 = TexturedVertexData::from_command(
                        self.fifo[9],
                        self.fifo[10],
                        self.fifo[11],
                    );

                    self.textured_triangle(
                        v2,
                        v3,
                        v4,
                        clut as u16,
                        semi_transparent,
                        TextureBlendingMode::Shaded,
                    );
                }
            }
            0x40 | 0x42 | 0x48 | 0x4a => {
                // Monochrome line or polyline. Opaque/semi-transparent.
                let semi_transparent = command & 2 != 0;
                let polyline = command & 8 != 0;

                let v1 = VertexData::from_command(self.fifo[0], self.fifo[1]);
                let v2 = VertexData::from_command(self.fifo[0], self.fifo[2]);

                let mut points = vec![v1, v2];

                if polyline {
                    // Take additional vertices, until one has bits 50005000 set
                    let mut i = 3;
                    while i < self.fifo_index {
                        let v = VertexData::from_command(self.fifo[0], self.fifo[i]);
                        if self.fifo[i] & 0x50005000 == 0x50005000 {
                            break; // Stop if we hit the end of the polyline
                        }
                        points.push(v);
                        i += 1;
                    }

                    if i == self.fifo_index {
                        panic!("Overflowing polyline command: FIFO index is {}, but no end of polyline found", self.fifo_index);
                    }
                }

                self.polyline(points, semi_transparent);
            }
            0x50 | 0x52 | 0x58 | 0x5a => {
                // Shaded line or polyline. Opaque/semi-transparent.
                let semi_transparent = command & 2 != 0;
                let polyline = command & 8 != 0;

                let v1 = VertexData::from_command(self.fifo[0], self.fifo[1]);
                let v2 = VertexData::from_command(self.fifo[2], self.fifo[3]);

                let mut points = vec![v1, v2];

                if polyline {
                    // Take additional vertices, until one has bits 50005000 set
                    let mut i = 4;
                    while i < self.fifo_index {
                        let v = VertexData::from_command(self.fifo[i], self.fifo[i + 1]);
                        if self.fifo[i] & 0x50005000 == 0x50005000 {
                            break; // Stop if we hit the end of the polyline
                        }
                        points.push(v);
                        i += 2;
                    }

                    if i == self.fifo_index {
                        panic!("Overflowing polyline command: FIFO index is {}, but no end of polyline found", self.fifo_index);
                    }
                }

                self.polyline(vec![v1, v2], semi_transparent);
            }
            0x60..=0x7f => {
                // Rectangle command. Opaque/semi-transparent.
                let semi_transparent = command & 2 != 0;
                let textured = command & 4 != 0;

                let mut v0 = VertexData::from_command(self.fifo[0], self.fifo[1]);

                let (w, h) = match (command >> 3) & 3 {
                    0 => {
                        let line = if textured {
                            self.fifo[3]
                        } else {
                            self.fifo[2]
                        };

                        let w = line & 0x3ff;
                        let h = (line >> 16) & 0x1ff;
                        (w, h)
                    }
                    1 => (1, 1),
                    2 => (8, 8),
                    3 => (16, 16),
                    _ => unreachable!()
                };

                v0.vertex.x = (v0.vertex.x + self.x_offset) & 0x3ff;
                v0.vertex.y = (v0.vertex.y + self.y_offset) & 0x1ff;

                let blending = if command & 1 != 0 {
                    TextureBlendingMode::Raw
                } else {
                    TextureBlendingMode::Modulated
                };

                let (clut_x, clut_y) = if textured {
                    let clut = (self.fifo[2] >> 16) as usize; // CLUT address
                    ((clut & 0x3f) * 16, (clut >> 6) & 0x1ff)
                } else {
                    (0, 0)
                };

                let tex_coords = if textured {
                    TextureCoords::from_command(self.fifo[2])
                } else {
                    TextureCoords::new(0, 0)
                };

                // if command == 0x65 {
                //     print!("[GPU] {command:02x}: v0: {v0:?} w: {w} h: {h} ");
                //     println!("\tTex coords: {tex_coords:?} Clut: {clut_x}, {clut_y}");
                // }

                for yoff in 0..h {
                    for xoff in 0..w {
                        let x = (v0.vertex.x + xoff as isize) as usize;
                        let y = (v0.vertex.y + yoff as isize) as usize;

                        // Skip pixels outside the clipping rectangle
                        if x < self.min_x || x > self.max_x || y < self.min_y || y > self.max_y {
                            continue;
                        }

                        let idx = y * 1024 + x;

                        if textured {
                            let tex_coords = TextureCoords::new(
                                (tex_coords.x + xoff as usize) & 0xff,
                                (tex_coords.y + yoff as usize) & 0xff
                            );
                            let texel = self.sample_texture(tex_coords);

                            let tex_color = match self.tex_page_depth {
                                // 4-bit texture mode, the texel is an index into the CLUT
                                0 | 1 => Color::from_bgr555(
                                    self.vram[clut_y * 1024 + clut_x + texel as usize],
                                ),
                                2 | 3 => Color::from_bgr555(texel),
                                _ => unreachable!(),
                            };

                            if tex_color.r == 0 && tex_color.g == 0 && tex_color.b == 0 {
                                // Color black has a special treatment.
                                // when the transparency (bit 15) is clear, the texel is ignored

                                // If the transparency bit of the texel is set, it is rendered.
                                // That bit is ignored if the command is "opaque" (semi_transparent is false).
                                // Otherwise the black is blended with the VRAM color.

                                if !tex_color.transparent {
                                    continue;
                                }

                                // TODO: Handle semi-transparency blending
                            }

                            let color = match blending {
                                TextureBlendingMode::Raw => {
                                    // Use the texture color directly
                                    tex_color
                                }
                                TextureBlendingMode::Shaded => unreachable!(),
                                TextureBlendingMode::Modulated => {
                                    // Modulate the texture color with the main color
                                    tex_color.modulate(&v0.color)
                                }
                            };

                            self.vram[idx] = color.to_bgr555();
                        } else {
                            self.vram[idx] = v0.color.to_bgr555();
                        }
                    }
                }
            }
            0xe1 => {
                self.tex_page_x = ((self.fifo[0] & 0xf) * 64) as usize; // X position
                self.tex_page_y = (((self.fifo[0] >> 4) & 1) * 256) as usize; // Y position
                self.opacity_flag = ((self.fifo[0] >> 5) & 0x3) as usize;
                self.tex_page_depth = ((self.fifo[0] >> 7) & 0x3) as usize; // Depth
                self.enable_dithering = self.fifo[0] & 0x200 != 0;

                // println!("Texpage: {:08x}", self.fifo[0]);
            }
            0xe2 => {
                let mask_x = self.fifo[0] & 0x1f;
                let mask_y = (self.fifo[0] >> 5) & 0x1f;
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

                // println!(
                //     "X offset: {}, Y offset: {}",
                //     self.x_offset, self.y_offset
                // );
            }
            0xe6 => {
                // println!("Mask bit settings: {:#x}", self.fifo[0]);
            }
            _ => {
                panic!(
                    "[GPU] Unknown command {:#x} with {} words in FIFO",
                    command, self.fifo_index
                );
            }
        }
    }

    fn update_texture_page(&mut self, page: u32) {
        self.tex_page_x = ((page & 0xf) * 64) as usize; // Texture page X position
        self.tex_page_y = (((page >> 4) & 0x1) * 256) as usize; // Texture page Y position
        self.opacity_flag = ((page >> 5) & 0x3) as usize; // Opacity flag
        self.tex_page_depth = ((page >> 7) & 0x3) as usize; // Texture page depth
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
        // Convert vertex coordinates to (x, y) pairs
        let (x0, y0) = v1.vertex.explode();
        let (x1, y1) = v2.vertex.explode();
        let (x2, y2) = v3.vertex.explode();

        // compute the distance between x0 and x1:
        let dx01 = (x0 as isize - x1 as isize).abs() as usize;
        let dx12 = (x1 as isize - x2 as isize).abs() as usize;
        let dx20 = (x2 as isize - x0 as isize).abs() as usize;
        let dy01 = (y0 as isize - y1 as isize).abs() as usize;
        let dy12 = (y1 as isize - y2 as isize).abs() as usize;
        let dy20 = (y2 as isize - y0 as isize).abs() as usize;

        if dx01 >= 1024
            || dx12 >= 1024
            || dx20 >= 1024
            || dy01 >= 512
            || dy12 >= 512
            || dy20 >= 512
        {
            // Skip rendering if any two vertices are too far apart
            return;
        }

        // TODOs:
        // - Handle semi-transparency
        // - Do not render if any two vertices are 1024 or more horizontal
        //   pixels apart or 512 or more vertical pixels apart

        let min_x = x0.min(x1).min(x2);
        let max_x = x0.max(x1).max(x2);
        let min_y = y0.min(y1).min(y2);
        let max_y = y0.max(y1).max(y2);

        let area = Self::edge_fn(v1.vertex, v2.vertex, v3.vertex);

        let (v1, v2, area) = if area < 0.0 {
            // Swap vertices if the area is negative (to ensure counter-clockwise winding)
            (v2, v1, -area)
        } else {
            (v1, v2, area)
        };

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let w0 = Self::edge_fn(v2.vertex, v3.vertex, Vertex::new(x, y));
                let w1 = Self::edge_fn(v3.vertex, v1.vertex, Vertex::new(x, y));
                let w2 = Self::edge_fn(v1.vertex, v2.vertex, Vertex::new(x, y));

                if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                    let alpha = w0 / area;
                    let beta = w1 / area;
                    let gamma = w2 / area;

                    let color = Color::lerp3(
                        v1.color, v2.color, v3.color, alpha, beta, gamma,
                    );

                    let x = ((x as isize + self.x_offset) & 0x3ff) as usize; // Wrap around at 1024
                    let y = ((y as isize + self.y_offset) & 0x1ff) as usize; // Wrap around at 512

                    if x < self.min_x
                        || x > self.max_x
                        || y < self.min_y
                        || y > self.max_y
                    {
                        continue; // Skip pixels outside the clipping rectangle
                    }

                    let color = if self.enable_dithering && !force_dither_off {
                        self.dither(x, y, color)
                    } else {
                        color
                    };

                    self.vram[y * 1024 + x] = color.to_bgr555();
                }
            }
        }
    }

    fn textured_triangle(
        &mut self,
        v1: TexturedVertexData,
        v2: TexturedVertexData,
        v3: TexturedVertexData,
        clut: u16,
        semi_transparent: bool,
        blending_mode: TextureBlendingMode,
    ) {
        // Convert vertex coordinates to (x, y) pairs
        let (x0, y0) = v1.vertex.explode();
        let (x1, y1) = v2.vertex.explode();
        let (x2, y2) = v3.vertex.explode();

        // compute the distance between x0 and x1:
        let dx01 = (x0 as isize - x1 as isize).abs() as usize;
        let dx12 = (x1 as isize - x2 as isize).abs() as usize;
        let dx20 = (x2 as isize - x0 as isize).abs() as usize;
        let dy01 = (y0 as isize - y1 as isize).abs() as usize;
        let dy12 = (y1 as isize - y2 as isize).abs() as usize;
        let dy20 = (y2 as isize - y0 as isize).abs() as usize;

        if dx01 >= 1024
            || dx12 >= 1024
            || dx20 >= 1024
            || dy01 >= 512
            || dy12 >= 512
            || dy20 >= 512
        {
            println!(
                "[GPU] Skipping triangle rendering: vertices too far apart"
            );
            // Skip rendering if any two vertices are too far apart
            return;
        }

        // TODOs:
        // - Handle semi-transparency
        // - Do not render if any two vertices are 1024 or more horizontal pixels apart or 512 or more vertical pixels apart

        let min_x = x0.min(x1).min(x2);
        let max_x = x0.max(x1).max(x2);
        let min_y = y0.min(y1).min(y2);
        let max_y = y0.max(y1).max(y2);

        let area = Self::edge_fn(v1.vertex, v2.vertex, v3.vertex);

        let (v1, v2, area) = if area < 0.0 {
            // Swap vertices if the area is negative (to ensure counter-clockwise winding)
            (v2, v1, -area)
        } else {
            (v1, v2, area)
        };

        let clut_x = (clut as usize & 0x3f) * 16; // CLUT X position
        let clut_y = (clut as usize >> 6) & 0x1ff;

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let w0 = Self::edge_fn(v2.vertex, v3.vertex, Vertex::new(x, y));
                let w1 = Self::edge_fn(v3.vertex, v1.vertex, Vertex::new(x, y));
                let w2 = Self::edge_fn(v1.vertex, v2.vertex, Vertex::new(x, y));

                if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                    let alpha = w0 / area;
                    let beta = w1 / area;
                    let gamma = w2 / area;

                    let texture_coords = TextureCoords::lerp3(
                        v1.texture_coords,
                        v2.texture_coords,
                        v3.texture_coords,
                        alpha,
                        beta,
                        gamma,
                    );

                    let texel = self.sample_texture(texture_coords);
                    let tex_color = match self.tex_page_depth {
                        // 4-bit texture mode, the texel is an index into the CLUT
                        0 | 1 => Color::from_bgr555(
                            self.vram[clut_y * 1024 + clut_x + texel as usize],
                        ),
                        2 | 3 => Color::from_bgr555(texel),
                        _ => unreachable!(),
                    };

                    if tex_color.r == 0 && tex_color.g == 0 && tex_color.b == 0
                    {
                        // Color black has a special treatment.
                        // when the transparency (bit 15) is clear, the texel is ignored

                        // If the transparency bit of the texel is set, it is rendered.
                        // That bit is ignored if the command is "opaque" (semi_transparent is false).
                        // Otherwise the black is blended with the VRAM color.

                        if !tex_color.transparent {
                            continue;
                        }

                        // TODO: Handle semi-transparency blending
                    }

                    let color = match blending_mode {
                        TextureBlendingMode::Raw => {
                            // Use the texture color directly
                            tex_color
                        }
                        TextureBlendingMode::Shaded => {
                            // Blend the texture color with the vertex colors
                            let color = Color::lerp3(
                                v1.color, v2.color, v3.color, alpha, beta,
                                gamma,
                            );

                            tex_color.multiply(&color)
                        }
                        TextureBlendingMode::Modulated => {
                            // Modulate the texture color with the main color
                            tex_color.modulate(&v1.color)
                        }
                    };

                    let x = ((x as isize + self.x_offset) & 0x3ff) as usize; // Wrap around at 1024
                    let y = ((y as isize + self.y_offset) & 0x1ff) as usize; // Wrap around at 512

                    if x < self.min_x
                        || x > self.max_x
                        || y < self.min_y
                        || y > self.max_y
                    {
                        continue; // Skip pixels outside the clipping rectangle
                    }

                    let color = if self.enable_dithering
                        && blending_mode != TextureBlendingMode::Raw
                    {
                        self.dither(x, y, color)
                    } else {
                        color
                    };

                    self.vram[y * 1024 + x] = color.to_bgr555();
                }
            }
        }
    }

    fn polyline(
        &mut self,
        vertices: Vec<VertexData>,
        semi_transparent: bool,
    ) {
        // Draw a polyline by connecting the vertices with lines
        for i in 0..vertices.len() - 1 {
            let v1 = vertices[i];
            let v2 = vertices[i + 1];
            self.line(v1, v2, semi_transparent);
        }
    }

    fn line(
        &mut self,
        v1: VertexData,
        v2: VertexData,
        semi_transparent: bool
    ) {
        println!(
            "[GPU] Drawing line from ({}, {}) to ({}, {})",
            v1.vertex.x, v1.vertex.y, v2.vertex.x, v2.vertex.y
        );
    }

    fn edge_fn(v1: Vertex, v2: Vertex, v3: Vertex) -> f64 {
        let x1 = v1.x as f64 + 0.5;
        let y1 = v1.y as f64 + 0.5;
        let x2 = v2.x as f64 + 0.5;
        let y2 = v2.y as f64 + 0.5;
        let x3 = v3.x as f64 + 0.5;
        let y3 = v3.y as f64 + 0.5;

        (x2 - x1) * (y3 - y1) - (y2 - y1) * (x3 - x1)
    }

    fn sample_texture(&self, coords: TextureCoords) -> u16 {
        let tex_y = self.tex_page_y + coords.y;
        let mut tex_x = self.tex_page_x;

        match self.tex_page_depth {
            0 => {
                // 4-bit mode. Each VRAM halfwords contains 4 values, each 4 bits.
                // This value is an index in the CLUT, which is a 16 16-bit color palette.
                // The X coordinate indices the individual nibbles in the halfword.
                tex_x += coords.x / 4;
            }
            1 => {
                // 8-bit mode. Each VRAM halfword contains 2 values, each 8 bits.
                // This value is an index in the CLUT, which is a 256 16-bit color palette.
                // The X coordinate indices the individual bytes in the halfword.
                tex_x += coords.x / 2;
            }
            2 | 3 => {
                // 16-bit direct mode. Each VRAM halfword contains a 16-bit color.
                tex_x += coords.x; // Directly use the X coordinate
            }
            _ => {
                unreachable!(); // Invalid texture depth
            }
        }

        // Get the texture byte from VRAM
        let tex_halfword = self.vram[tex_y * 1024 + tex_x];
        match self.tex_page_depth {
            0 => {
                // 4-bit mode
                (tex_halfword >> ((coords.x % 4) * 4)) & 0xf
            }
            1 => {
                // 8-bit mode
                (tex_halfword >> ((coords.x % 2) * 8)) & 0xff
            }
            2 | 3 => {
                // 16-bit direct mode
                tex_halfword
            }
            _ => unreachable!(), // Invalid texture depth
        }
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

    const DITHER_MATRIX: [[i16; 4]; 4] = [
        [-4, 0, -3, 1],
        [2, -2, 3, -1],
        [-3, 1, -4, 0],
        [3, -1, 2, -2],
    ];

    fn dither(&self, x: usize, y: usize, color: Color) -> Color {
        let dither_value = Self::DITHER_MATRIX[y % 4][x % 4] + 4; // Shift to positive range
        let r = (color.r as i16 + dither_value).clamp(0, 0xff) as u8;
        let g = (color.g as i16 + dither_value).clamp(0, 0xff) as u8;
        let b = (color.b as i16 + dither_value).clamp(0, 0xff) as u8;

        Color::new(r.min(255), g.min(255), b.min(255), color.transparent)
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
    color: Color,
    vertex: Vertex,
}

impl VertexData {
    fn from_command(color_word: u32, vertex_word: u32) -> Self {
        let color = Color::from_command(color_word);
        let vertex = Vertex::from_command(vertex_word);
        Self { color, vertex }
    }
}

#[derive(Debug, Clone, Copy)]
struct TexturedVertexData {
    color: Color,
    vertex: Vertex,
    texture_coords: TextureCoords,
}

impl TexturedVertexData {
    fn from_command(
        color_word: u32,
        vertex_word: u32,
        texture_word: u32,
    ) -> Self {
        let color = Color::from_command(color_word);
        let vertex = Vertex::from_command(vertex_word);
        let texture_coords = TextureCoords::from_command(texture_word);
        Self {
            color,
            vertex,
            texture_coords,
        }
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

        Self {
            x: x as isize,
            y: y as isize,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
    transparent: bool,
}

impl Color {
    fn new(r: u8, g: u8, b: u8, t: bool) -> Self {
        Self {
            r,
            g,
            b,
            transparent: t,
        }
    }

    fn to_rgb888(&self) -> (u8, u8, u8) {
        (self.r, self.g, self.b)
    }

    fn from_bgr555(color: u16) -> Self {
        let r = ((color & 0x1f)) as u16; // 5 bits for red
        let g = (((color >> 5) & 0x1f)) as u16; // 5 bits for green
        let b = (((color >> 10) & 0x1f)) as u16; // 5 bits for blue
        let t = (color & 0x8000) != 0; // Check if the color is semi-transparent

        Self {
            r: (r as f32 / 31.0 * 255.0) as u8,
            g: (g as f32 / 31.0 * 255.0) as u8,
            b: (b as f32 / 31.0 * 255.0) as u8,
            transparent: t,
        }
    }

    fn lerp3(
        c0: Color,
        c1: Color,
        c2: Color,
        w0: f64,
        w1: f64,
        w2: f64,
    ) -> Color {
        let r = c0.r as f64 * w0 + c1.r as f64 * w1 + c2.r as f64 * w2;
        let g = c0.g as f64 * w0 + c1.g as f64 * w1 + c2.g as f64 * w2;
        let b = c0.b as f64 * w0 + c1.b as f64 * w1 + c2.b as f64 * w2;

        Color {
            r: r.clamp(0.0, 255.0) as u8,
            g: g.clamp(0.0, 255.0) as u8,
            b: b.clamp(0.0, 255.0) as u8,
            transparent: c0.transparent || c1.transparent || c2.transparent,
        }
    }

    fn from_command(rgb: u32) -> Self {
        let r = (rgb & 0xff) as u8;
        let g = ((rgb >> 8) & 0xff) as u8;
        let b = ((rgb >> 16) & 0xff) as u8;

        Self::new(r, g, b, false)
    }

    fn to_bgr555(&self) -> u16 {
        let r = (self.r >> 3) & 0x1f; // 5 bits for red
        let g = (self.g >> 3) & 0x1f; // 5 bits for green
        let b = (self.b >> 3) & 0x1f; // 5 bits for blue
        let t = if self.transparent { 0x8000 } else { 0 };

        (r as u16) | ((g as u16) << 5) | ((b as u16) << 10) | t
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

        Color::new(r, g, b, self.transparent)
    }

    fn multiply(&self, other: &Color) -> Color {
        let r = ((self.r as f64 * other.r as f64) as u16 >> 8) as u8;
        let g = ((self.g as f64 * other.g as f64) as u16 >> 8) as u8;
        let b = ((self.b as f64 * other.b as f64) as u16 >> 8) as u8;

        Color::new(r, g, b, self.transparent || other.transparent)
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

    fn lerp3(
        v1: TextureCoords,
        v2: TextureCoords,
        v3: TextureCoords,
        alpha: f64,
        beta: f64,
        gamma: f64,
    ) -> Self {
        let x = (v1.x as f64 * alpha + v2.x as f64 * beta + v3.x as f64 * gamma)
            .round() as usize;
        let y = (v1.y as f64 * alpha + v2.y as f64 * beta + v3.y as f64 * gamma)
            .round() as usize;

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
