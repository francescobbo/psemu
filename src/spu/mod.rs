mod gauss;
mod voice;

use crate::{bus::AccessSize, spu::voice::{AdsrEnvelope, AdsrPhase}};

#[derive(Default, Copy, Clone, Debug)]
struct Voice {
    n: usize,

    envelope: AdsrEnvelope, // ADSR envelope for this voice

    sample_rate: u16,
    start_address: u32,
    repeat_address: u32,

    current_address_internal: u32,

    decode_buffer: [i16; 32], // Buffer for decoded ADPCM samples

    pitch_counter: u16,     // Pitch counter for this voice
    current_buffer_idx: usize, // Current index in the decode buffer
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

    spucnt: u16,
    key_on_register: u32,
    key_off_register: u32, // Key OFF register
    transfer_control: u16,
    reverb_mode: u32, // Reverb mode register
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
            voices: (0..24)
                .map(|n| Voice::new(n)) // Create 24 voices
                .collect(),
            spucnt: 0,                // SPUCNT register, default value
            key_on_register: 0,       // Key ON register, default value
            key_off_register: 0,      // Key OFF register, default value
            transfer_control: 0, // Transfer control register, default value
            reverb_mode: 0,      // Reverb mode register, default value
        }
    }

    pub fn tick(&mut self) -> (f32, f32) {
        // Process each voice
        for voice in &mut self.voices {
            // Clock the voice to update its sample
            voice.clock(&self.ram);
        }

        let mut mixed_sample_left = 0i32;
        let mut mixed_sample_right = 0i32;

        for voice in &self.voices {
            let sample = voice.get_sample();

            // Mix the current sample from the voice
            mixed_sample_left += i32::from(sample.0);
            mixed_sample_right += i32::from(sample.1);
        }

        // Clamp the sums to signed 16-bit
        let clamped_l = mixed_sample_left.clamp(-0x8000, 0x7FFF) as i16;
        let clamped_r = mixed_sample_right.clamp(-0x8000, 0x7FFF) as i16;

        // Apply main volume after mixing
        let output_l = apply_volume(clamped_l, self.volume_left as i16);
        let output_r = apply_volume(clamped_r, self.volume_right as i16);

        // Convert to f32
        (output_l as f32 / 32768.0,
         output_r as f32 / 32768.0)
    }

    pub fn read(&self, address: u32, size: AccessSize) -> u32 {
        match address {
            0x1f801c00..=0x1f801d7f => {
                let voice_index = (address - 0x1f801c00) / 16;

                match address & 0xf {
                    0 => {
                        // Volume left
                        self.voices[voice_index as usize].volume_left as u32
                    }
                    2 => {
                        // Volume right
                        self.voices[voice_index as usize].volume_right as u32
                    }
                    4 => {
                        // Sample rate
                        self.voices[voice_index as usize].sample_rate as u32
                    }
                    6 => {
                        // Start address
                        self.voices[voice_index as usize].start_address >> 3
                    }
                    8 => {
                        self.voices[voice_index as usize]
                            .envelope
                            .read_low() as u32 // Read ADSR register
                    }
                    0xa => {
                        self.voices[voice_index as usize]
                            .envelope
                            .read_high() as u32 // Read ADSR2 register
                    }
                    0xc => {
                        // ADS current volume
                        // println!("Reading current address is not implemented");
                        self.voices[voice_index as usize]
                            .envelope
                            .level as u32
                    }
                    0xe => {
                        // Repeat address
                        self.voices[voice_index as usize].repeat_address >> 3
                    }
                    _ => {
                        println!(
                            "Reading from voice register at address {:#x} is not implemented",
                            address
                        );
                        0
                    }
                }
            }
            0x1f801d88 => self.key_on_register & 0xffff, // Read lower 16 bits of key on register
            0x1f801d8a => (self.key_on_register >> 16) & 0xffff, // Read upper 16 bits of key on register
            0x1f801d8c => self.key_off_register & 0xffff,
            0x1f801d8e => self.key_off_register >> 16,
            0x1f801d98 => {
                // Reverb mode, lower 16 bits
                self.reverb_mode & 0xffff
            }
            0x1f801d9a => {
                // Reverb mode, upper 16 bits
                self.reverb_mode >> 16
            }
            0x1f801da6 => {
                // Sound RAM data transfer address
                self.data_start_address >> 3
            }
            0x1f801daa => self.spucnt as u32,
            0x1f801dac => self.transfer_control as u32, // IF THIS IS NOT IMPLEMENTED, THE BIOS WILL NOT SEND THE FULL BOOT SEQUENCE
            0x1f801dae => {
                let b7 = (self.spucnt & 0x20) << 2;
                ((self.spucnt & 0x1f) | b7) as u32
            }
            0x1f801db8 => {
                // Current main volume left/right (read-only)
                // This is a read-only register that returns the current main volume
                // left and right, which is not implemented in this example.
                (self.volume_left as u32) | ((self.volume_right as u32) << 16)
            }
            _ => {
                println!(
                    "Reading from SPU register at address {:#x} is not implemented",
                    address
                );
                0
            }
        }
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
                        // println!(
                        //     "Setting volume left for voice {} to {}",
                        //     voice_index,
                        //     self.voices[voice_index as usize].volume_left
                        // );
                    }
                    2 => {
                        // Volume right
                        self.voices[voice_index as usize].volume_right =
                            (value & 0xffff) as u16;
                        // println!(
                        //     "Setting volume left for voice {} to {}",
                        //     voice_index,
                        //     self.voices[voice_index as usize].volume_right
                        // );
                    }
                    4 => {
                        self.voices[voice_index as usize].sample_rate =
                            (value & 0xffff) as u16;
                        // println!(
                        //     "Setting sample rate for voice {} to {}",
                        //     voice_index,
                        //     self.voices[voice_index as usize].sample_rate
                        // );
                    }
                    6 => {
                        self.voices[voice_index as usize].start_address =
                            (value & 0xffff) << 3;
                        // println!(
                        //     "Setting start address for voice {} to {:#x}",
                        //     voice_index,
                        //     self.voices[voice_index as usize].start_address
                        // );
                    }
                    8 => {
                        self.voices[voice_index as usize]
                            .envelope
                            .write_low(value as u16);
                    }
                    0xa => {
                        self.voices[voice_index as usize]
                            .envelope
                            .write_high(value as u16);
                    }
                    0xc => {
                        // ADS current volume
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
                    _ => {
                        println!(
                            "Writing to voice register at address {:#x} is not implemented",
                            address
                        );
                    }
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
                assert!(size == AccessSize::HalfWord);
                // Voice Key ON register, part 1, 16-bit.
                // Bits 0-23 are used to start voices.

                for i in 0..16 {
                    if (value & (1 << i)) != 0 {
                        self.voices[i].key_on(&self.ram);
                    }
                }

                self.key_on_register =
                    self.key_on_register & 0xffff_0000 | (value & 0xffff);
            }
            0x1f801d8a => {
                assert!(size == AccessSize::HalfWord);
                // Voice Key ON register, part 2, 16-bit.
                // Bits 0-23 are used to start voices.

                for i in 0..8 {
                    if (value & (1 << i)) != 0 {
                        self.voices[16 + i].key_on(&self.ram);
                    }
                }

                self.key_on_register =
                    self.key_on_register & 0x0000_ffff | (value << 16);
            }
            0x1f801d8c => {
                assert!(size == AccessSize::HalfWord);
                // Voice Key OFF register, 32-bit.
                // Bits 0-23 are used to stop voices.

                for i in 0..16 {
                    if (value & (1 << i)) != 0 {
                        self.voices[i].key_off();
                    }
                }

                self.key_off_register =
                    self.key_off_register & 0xffff_0000 | (value & 0xffff);
            }
            0x1f801d8e => {
                assert!(size == AccessSize::HalfWord);
                // Voice Key OFF register, part 2, 16-bit.
                // Bits 0-23 are used to stop voices.

                for i in 0..8 {
                    if (value & (1 << i)) != 0 {
                        self.voices[16 + i].key_off();
                    }
                }

                self.key_off_register =
                    self.key_off_register & 0x0000_ffff | (value << 16);
            }
            0x1f801d90 => {
                // Voice Pitch modulation enable flags. Bits 1-16
                // are used to enable pitch modulation. For channel x,
                // uses channel x-1's amplitude as pitch modulation.
                // println!(
                //     "Warning: Writing to voice pitch modulation enable flags at address {:#x}",
                //     address
                // );
            }
            0x1f801d92 => {
                // Voice Pitch modulation flags. Bits 16-23.
            }
            0x1f801d94 => {
                // Voice Noise mode enable flags. Bits 0-15. 1 means noise, 0 is ADPCM.
                // println!(
                //     "Warning: Writing to voice noise mode enable flags at address {:#x}",
                //     address
                // );
            }
            0x1f801d96 => {
                // Voice Noise mode flags. Bits 16-23.
            }
            0x1f801d98 => {
                // Reverb mode
                self.reverb_mode =
                    self.reverb_mode & 0xffff_0000 | (value & 0xffff);
            }
            0x1f801d9a => {
                self.reverb_mode =
                    self.reverb_mode & 0x0000_ffff | (value << 16);
            }
            0x1f801da0 => {
                // unused
                // println!(
                //     "Warning: Writing to unused SPU register at address {:#x}",
                //     address
                // );
            }
            0x1f801da2 => {
                // Reverb work area start address in sound RAM.
                // This is the address in sound RAM where the reverb work area starts.
                // println!(
                //     "Warning: Writing to reverb work area start address at {:#x} is not implemented",
                //     address
                // );
            }
            0x1f801da4 => {
                // Address in sound buffer (divided by 8) that raises an interrupt
                // println!(
                //     "Warning: Writing to sound buffer interrupt address at {:#x} is not implemented",
                //     address
                // );
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
                        if self.data_start_address_internal >= 512 * 1024 {
                            // Wrap around if we exceed sound RAM size
                            self.data_start_address_internal = 0;
                        }
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
                self.spucnt = (value & 0xffff) as u16;
                // println!(
                //     "[SPU] SPUCNT write value: {value:#x}",
                // );
            }
            0x1f801dac => {
                // Sound RAM data transfer control register.
                println!("[SPU] Transfer control: {value:#x}");

                self.transfer_control = (value & 0xffff) as u16;
            }
            0x1f801dae => {
                // SPUSTAT
                println!("[SPU] Ignoring write to SPUSTAT",);
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
                // println!(
                //     "Warning: Writing to internal voice register at address {:#x} is not recommended.",
                //     address
                // );
            }
            0x1f801dc0..=0x1f801dff => {
                // REVERB registers, not implemented
                // println!(
                //     "Warning: Writing to unused SPU register at address {:#x} is not implemented",
                //     address
                // );
            }
            _ => {
                // Handle other addresses or ignore
                // println!(
                //     "Warning: Writing to unimplemented SPU register at address {:#x}",
                //     address
                // );
            }
        }
    }

    fn decode_adpcm_block(
        block: &[u8],
        decoded: &mut [i16; 32],
    ) {
        // First byte is a header byte specifying the shift value (bits 0-3) and the filter value (bits 4-6).
        // A shift value of 13-15 is invalid and behaves the same as shift=9
        let shift = block[0] & 0x0F;
        let shift = if shift > 12 { 9 } else { shift };

        // Filter values can only range from 0 to 4
        let filter = std::cmp::min(4, (block[0] >> 4) & 0x07);

        for i in 0..4 {
            decoded[i] = decoded[28 + i];
        }

        // The remaining 14 bytes are encoded sample values
        for sample_idx in 0..28 {
            // Read the raw 4-bit sample value from the block.
            // Samples are stored little-endian within a byte
            let sample_byte = block[2 + sample_idx / 2];
            let sample_nibble = (sample_byte >> (4 * (sample_idx % 2))) & 0x0f;

            // Sign extend from 4 bits to 32 bits
            let raw_sample: i32 = (((sample_nibble as i8) << 4) >> 4).into();

            // Apply the shift; a shift value of N is decoded by shifting left (12 - N)
            let shifted_sample = raw_sample << (12 - shift);

            let old: i32 = decoded[sample_idx + 3].into();
            let older: i32 = decoded[sample_idx + 2].into();

            // Apply the filter formula.
            // In real code you can do this with tables instead of a match
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
            decoded[sample_idx + 4] = clamped_sample;
        }
    }
}

impl Voice {
    fn new(n: usize) -> Self {
        Voice {
            n,
            ..Default::default() // Initialize all fields to default values
        }
    }

    fn key_on(&mut self, ram: &[u8]) {
        if self.n == 0 {
            println!(
                "[SPU] Key ON for voice {} at address {:#x}",
                self.n, self.start_address
            );
        }

        self.current_address_internal = self.start_address;
        self.pitch_counter = 0;
        self.current_buffer_idx = 0;
        self.decode_next_block(ram);

        self.envelope.key_on();
    }

    fn key_off(&mut self) {
        self.envelope.key_off();
    }

    fn decode_next_block(&mut self, sound_ram: &[u8]) {
        let block = &sound_ram[self.current_address_internal as usize
            ..(self.current_address_internal + 16) as usize];

        Spu::decode_adpcm_block(block, &mut self.decode_buffer);

        // if self.n == 0 {
        //     println!(
        //         "[SPU] Decoding next block at address {:#x}. Data: {:?}, Res: {:?}",
        //         self.current_address_internal, block, self.decode_buffer
        //     );
        // }

        let loop_end = block[1] & 1 != 0;
        let loop_repeat = block[1] & (1 << 1) != 0;
        let loop_start = block[1] & (1 << 2) != 0;

        if loop_start {
            // Start of loop, update repeat address
            self.repeat_address = self.current_address_internal;

            // println!(
            //     "Loop start at address {:#x}, repeat address set to {:#x}",
            //     self.current_address_internal, self.repeat_address
            // );
        }

        if loop_end {
            // End of loop, jump to start of loop
            self.current_address_internal = self.repeat_address;

            // println!(
            //     "Loop end at address {:#x}, jumping to repeat address {:#x}",
            //     self.current_address_internal, self.repeat_address
            // );

            if !loop_repeat {
                // End of non-repeating loop, immediately mute the voice
                self.envelope.level = 0;
                self.envelope.phase = AdsrPhase::Release;
            }
        } else {
            // Not end of loop, move to the next 16-byte block
            self.current_address_internal += 16;
            self.current_address_internal &= 0x7FFFF; // Wrap around at 512 KiB
        }
    }

    fn clock(&mut self, sound_ram: &[u8]) {
        // Increment pitch counter using the sample rate.
        // Effective sample rate cannot be larger than 0x4000 (176400 Hz)
        let pitch_counter_step = std::cmp::min(0x4000, self.sample_rate);
        // In a full implementation, pitch modulation would be applied right here
        self.pitch_counter += pitch_counter_step;

        self.envelope.clock();

        // println!(
        //     "[SPU] Clocking voice: pitch_counter: {:#x}",
        //     pitch_counter_step,
        // );

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

        let sample = gauss::gaussian(self.last4_samples(), self.pitch_counter);
        
        // if self.n == 0 {
        //     println!("Gaussian sample: {:#06X}. Last4: {:?}, pc: {:#06X}",
        //                 sample, self.last4_samples(), self.pitch_counter);
        // }

        // Update current sample.
        // In a full implementation, this is where sample interpolation and voice volume would be applied
        self.current_sample = sample;
    }

    fn get_sample(&self) -> (i16, i16) {
        let envelope_sample = apply_volume(self.current_sample, self.envelope.level);

        if self.volume_left & 0x8000 != 0 {
            panic!("Volume sweep left!")
        } else if self.volume_right & 0x8000 != 0 {
            panic!("Volume sweep right!")
        }

        let actual_volume_left = (self.volume_left << 1) as i16;
        let actual_volume_right = (self.volume_right << 1) as i16;

        let output_l = apply_volume(envelope_sample, actual_volume_left);
        let output_r = apply_volume(envelope_sample, actual_volume_right);

        // if self.n == 0 {
        //     println!("V0: raw_sample={:#06X} pitch_counter={:#06X} sample_rate={:#06X} adrs={:#06X} vl={:#06X} vr={:#06X} sample_l={:#06X} sample_r={:#06X}",
        //              self.current_sample, self.pitch_counter, self.sample_rate, self.envelope.level, actual_volume_left, actual_volume_right, output_l, output_r);
        // }

        (output_l, output_r)
    }

    fn last4_samples(&self) -> [i16; 4] {
        self.decode_buffer[self.current_buffer_idx..= self.current_buffer_idx + 3]
            .try_into()
            .expect("Decode buffer should always have at least 4 samples")
    }
}

fn apply_volume(sample: i16, volume: i16) -> i16 {
    ((sample as i32 * volume as i32) >> 15) as i16
}