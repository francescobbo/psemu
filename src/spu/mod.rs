use std::os::raw::c_void;

use crate::{bus::AccessSize, ram};

#[derive(Default, Copy, Clone, Debug)]
struct Voice {
    sample_rate: u16,
    start_address: u32,
    repeat_address: u32,

    current_address_internal: u32,

    key_on: bool,

    decode_buffer: [i16; 28], // Buffer for decoded ADPCM samples

    pitch_counter: u16,     // Pitch counter for this voice
    current_buffer_idx: u8, // Current index in the decode buffer
    current_sample: i16,    // Current sample being played

    volume_left: u16,  // Volume for left channel
    volume_right: u16, // Volume for right channel
}

pub struct Spu {
    volume_left: u16,
    volume_right: u16,
    reverb_vol_left: u16,
    reverb_vol_right: u16,

    data_start_address: u32,
    data_start_address_internal: u32, // Internal address for data transfer

    ram: Vec<u8>,       // Sound RAM, 512 KiB
    voices: Vec<Voice>, // 24 voices
}

impl Spu {
    pub fn new() -> Self {
        Spu {
            volume_left: 0,                     // Default volume left
            volume_right: 0,                    // Default volume right
            reverb_vol_left: 0,                 // Default reverb volume left
            reverb_vol_right: 0,                // Default reverb volume right
            data_start_address: 0,              // Default data start address
            data_start_address_internal: 0,     // Internal data start address
            ram: vec![0; 512 * 1024], // Initialize sound RAM with 512 KiB of zeroes
            voices: vec![Voice::default(); 24], // Initialize 24 voices
        }
    }

    pub fn tick(&mut self) -> f32 {
        // Process each voice
        for voice in &mut self.voices {
            if voice.key_on {
                // Clock the voice to update its sample
                voice.clock(&self.ram);
            }
        }

        let mut mixed_sample = 0i32;
        for voice in &self.voices {
            if voice.key_on {
                // Mix the current sample from the voice
                mixed_sample += i32::from(voice.current_sample / 4);
            }
        }

        // Convert to f32
        let output_sample = mixed_sample.clamp(-0x8000, 0x7FFF) as i16;
        let output_sample = output_sample as f32 / (0x8000 as f32);
        output_sample
    }

    pub fn read(&self, address: u32, size: AccessSize) -> u32 {
        0
    }

    pub fn write(&mut self, address: u32, value: u32, size: AccessSize) {
        assert!(address >= 0x1f801c00 && address < 0x1f802000);

        // Handle writing to SPU registers
        match address {
            0x1f801c00..=0x1f801d7f => {
                // Voice registers
                let voice_index = (address - 0x1f801c00) / 16;

                match address & 0xf {
                    0 => {
                        // Volume left
                        self.voices[voice_index as usize].volume_left =
                            (value & 0xffff) as u16;
                        println!(
                            "Setting volume left for voice {} to {}",
                            voice_index,
                            self.voices[voice_index as usize].volume_left
                        );
                    }
                    2 => {
                        // Volume right
                        self.voices[voice_index as usize].volume_right =
                            (value & 0xffff) as u16;
                        println!(
                            "Setting volume left for voice {} to {}",
                            voice_index,
                            self.voices[voice_index as usize].volume_right
                        );
                    }
                    4 => {
                        self.voices[voice_index as usize].sample_rate =
                            (value & 0xffff) as u16;
                        println!(
                            "Setting sample rate for voice {} to {}",
                            voice_index,
                            self.voices[voice_index as usize].sample_rate
                        );
                    }
                    6 => {
                        self.voices[voice_index as usize].start_address =
                            (value & 0xffff) << 3;
                        println!(
                            "Setting start address for voice {} to {:#x}",
                            voice_index,
                            self.voices[voice_index as usize].start_address
                        );
                    }
                    8 => {
                        // ADSR register
                    }
                    0xa => {
                        // ADSR2
                    }
                    0xe => {
                        self.voices[voice_index as usize].repeat_address =
                            (value & 0xffff) << 3;
                        println!(
                            "Setting repeat address for voice {} to {:#x}",
                            voice_index,
                            self.voices[voice_index as usize].repeat_address
                        );
                    }
                    _ => unimplemented!(
                        "Writing to voice register at address {:#x} is not implemented",
                        address
                    ),
                }
            }
            0x1f801d80 => {
                // Volume left
                self.volume_left = (value & 0xffff) as u16;
            }
            0x1f801d82 => {
                // Volume right
                self.volume_right = (value & 0xffff) as u16;
            }
            0x1f801d84 => {
                // Reverb volume left
                self.reverb_vol_left = (value & 0xffff) as u16;
            }
            0x1f801d86 => {
                // Reverb volume right
                self.reverb_vol_right = (value & 0xffff) as u16;
            }
            0x1f801d88 => {
                // Voice Key ON register, 32-bit.
                // Bits 0-23 are used to start voices.

                for i in 0..24 {
                    if (value & (1 << i)) != 0 {
                        self.voices[i].key_on(&self.ram);
                    }
                }
            }
            0x1f801d8c => {
                // Voice Key OFF register, 32-bit.
                // Bits 0-23 are used to stop voices.

                for i in 0..24 {
                    if (value & (1 << i)) != 0 {
                        self.voices[i].key_on = false;
                    }
                }
            }
            0x1f801d90 => {
                // Voice Pitch modulation enable flags. Bits 1-23
                // are used to enable pitch modulation. For channel x,
                // uses channel x-1's amplitude as pitch modulation.
            }
            0x1f801d94 => {
                // Voice Noise mode enable flags. Bits 0-23. 1 means noise, 0 is ADPCM.
            }
            0x1f801da0 => {
                // unused
            }
            0x1f801da2 => {
                // Reverb work area start address in sound RAM.
            }
            0x1f801da4 => {
                // Address in sound buffer (divided by 8) that raises an interrupt
            }
            0x1f801da6 => {
                // Sound RAM data transfer address.
                self.data_start_address = (value & 0xffff) * 8;
                self.data_start_address_internal = self.data_start_address;
            }
            0x1f801da8 => {
                // Sound RAM data transfer queue.
                match size {
                    AccessSize::HalfWord => {
                        let ram = &mut self.ram
                            [self.data_start_address_internal as usize..];
                        let value = (value & 0xffff) as u16;
                        // Write the value to the sound RAM
                        ram[0] = (value & 0xff) as u8; // Lower byte
                        ram[1] = (value >> 8) as u8; // Upper byte
                        self.data_start_address_internal += 2;
                    }
                    AccessSize::Word => {
                        let ram = &mut self.ram
                            [self.data_start_address_internal as usize..];
                        // Write the value to the sound RAM
                        ram[0] = (value & 0xff) as u8; // Lower byte
                        ram[1] = ((value >> 8) & 0xff) as u8; // Second byte
                        ram[2] = ((value >> 16) & 0xff) as u8; // Third byte
                        ram[3] = ((value >> 24) & 0xff) as u8; // Upper byte
                        self.data_start_address_internal += 4;
                    }
                    AccessSize::Byte => {
                        unimplemented!(
                            "Byte access to sound RAM data transfer queue is not supported"
                        );
                    }
                }
            }
            0x1f801daa => {
                // SPUCNT
            }
            0x1f801dac => {
                // Sound RAM data transfer control register.
            }
            0x1f801dae => {
                // SPUSTAT
            }
            0x1f801db0 => {
                // CD Audio Input volume
            }
            0x1f801db4 => {
                // "External Audio Input" volume
            }
            0x1f801db8 => {
                // "Current" main volume left/right (read-only?)
            }
            0x1f801e00..=0x1f801e5f => {
                // Internal voice registers, should not be used by software
                println!(
                    "Warning: Writing to internal voice register at address {:#x} is not recommended.",
                    address
                );
            }
            _ => {
                // Handle other addresses or ignore
            }
        }
    }

    fn decode_adpcm_block(
        block: &[u8],
        decoded: &mut [i16; 28],
        mut old_sample: i16,
        mut older_sample: i16,
    ) {
        // First byte is a header byte specifying the shift value (bits 0-3) and the filter value (bits 4-6).
        // A shift value of 13-15 is invalid and behaves the same as shift=9
        let shift = block[0] & 0x0F;
        let shift = if shift > 12 { 9 } else { shift };

        // Filter values can only range from 0 to 4
        let filter = std::cmp::min(4, (block[0] >> 4) & 0x07);

        // The second byte is another header byte specifying loop flags; ignore that for now

        // The remaining 14 bytes are encoded sample values
        for sample_idx in 0..28 {
            // Read the raw 4-bit sample value from the block.
            // Samples are stored little-endian within a byte
            let sample_byte = block[2 + sample_idx / 2];
            let sample_nibble = (sample_byte >> (4 * (sample_idx % 2))) & 0x0F;

            // Sign extend from 4 bits to 32 bits
            let raw_sample: i32 = (((sample_nibble as i8) << 4) >> 4).into();

            // Apply the shift; a shift value of N is decoded by shifting left (12 - N)
            let shifted_sample = raw_sample << (12 - shift);

            // Apply the filter formula.
            // In real code you can do this with tables instead of a match
            let old = i32::from(old_sample);
            let older = i32::from(older_sample);
            let filtered_sample = match filter {
                // 0: No filtering
                0 => shifted_sample,
                // 1: Filter using previous sample
                1 => shifted_sample + (60 * old + 32) / 64,
                // 2-4: Filter using previous 2 samples
                2 => shifted_sample + (115 * old - 52 * older + 32) / 64,
                3 => shifted_sample + (98 * old - 55 * older + 32) / 64,
                4 => shifted_sample + (122 * old - 60 * older + 32) / 64,
                _ => unreachable!("filter was clamped to [0, 4]"),
            };

            // Finally, clamp to signed 16-bit
            let clamped_sample = filtered_sample.clamp(-0x8000, 0x7FFF) as i16;
            decoded[sample_idx] = clamped_sample;

            // Update sliding window for filter
            older_sample = old_sample;
            old_sample = clamped_sample;
        }
    }
}

impl Voice {
    fn key_on(&mut self, ram: &[u8]) {
        self.key_on = true;

        self.current_address_internal = self.start_address;
        self.pitch_counter = 0;
        self.current_buffer_idx = 0;
        self.decode_next_block(ram);
    }

    fn decode_next_block(&mut self, sound_ram: &[u8]) {
        let block = &sound_ram[self.current_address_internal as usize
            ..(self.current_address_internal + 16) as usize];

        let old = self.decode_buffer[27]; // old_sample
        let older = self.decode_buffer[26]; // older_sample

        Spu::decode_adpcm_block(block, &mut self.decode_buffer, old, older);

        let loop_end = block[1] & 1 != 0;
        let loop_repeat = block[1] & (1 << 1) != 0;
        let loop_start = block[1] & (1 << 2) != 0;

        if loop_start {
            // Start of loop, update repeat address
            self.repeat_address = self.current_address_internal;

            println!(
                "Loop start at address {:#x}, repeat address set to {:#x}",
                self.current_address_internal, self.repeat_address
            );
        }

        if loop_end {
            // End of loop, jump to start of loop
            self.current_address_internal = self.repeat_address;

            println!(
                "Loop end at address {:#x}, jumping to repeat address {:#x}",
                self.current_address_internal, self.repeat_address
            );

            if !loop_repeat {
                // End of non-repeating loop, immediately mute the voice
                self.volume_left = 0;
                self.volume_right = 0;
                self.key_on = false;
            }
        } else {
            // Not end of loop, move to the next 16-byte block
            self.current_address_internal += 16;
        }
    }

    fn clock(&mut self, sound_ram: &[u8]) {
        // Increment pitch counter using the sample rate.
        // Effective sample rate cannot be larger than 0x4000 (176400 Hz)
        let pitch_counter_step = std::cmp::min(0x4000, self.sample_rate);
        // In a full implementation, pitch modulation would be applied right here
        self.pitch_counter += pitch_counter_step;

        // Step through samples while pitch counter bits 12-15 are non-zero
        while self.pitch_counter >= 0x1000 {
            self.pitch_counter -= 0x1000;
            self.current_buffer_idx += 1;

            // Check if end of block was reached
            if self.current_buffer_idx == 28 {
                self.current_buffer_idx = 0;
                self.decode_next_block(sound_ram);
            }
        }

        // Update current sample.
        // In a full implementation, this is where sample interpolation and voice volume would be applied
        self.current_sample =
            self.decode_buffer[self.current_buffer_idx as usize];
    }
}
