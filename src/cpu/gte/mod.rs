use crate::cpu::Instruction;

mod division;
mod operations;

use bitfield::bitfield;

#[derive(Clone, Copy, Debug)]
struct RGB {
    r: u8,
    g: u8,
    b: u8,
    code: u8,
}

#[derive(Clone, Copy, Debug)]
struct XY {
    x: i16,
    y: i16,
}

type Matrix = [[i16; 3]; 3];

pub struct Gte {
    current_instruction: u32,

    cr: [u32; 32],

    rotation: Matrix,
    light: Matrix,
    color: Matrix,

    t: [i32; 4],
    b: [i32; 4],
    fc: [i32; 4],
    null: [i32; 4],

    /// Screen offset
    /// 32 bit, signed 15.16 fixed point
    ofx: i32,
    ofy: i32,

    /// Projection plane distance
    /// 16 bit integer, unsigned.
    h: u16,

    dqa: i16,
    dqb: i32,

    zsf3: i16,
    zsf4: i16,

    vectors: [[i16; 4]; 3],
    rgb: RGB,
    otz: u16,

    /// Intermediary registers
    /// 16 bit integers, signed.
    ir: [i16; 4],

    xy_fifo: [XY; 4],

    /// Screen Z-coordinate FIFO
    /// 16 bit integer, unsigned.
    z_fifo: [u16; 4],

    rgb_fifo: [RGB; 3],

    /// Math accumulators.
    /// 32 bit integers, signed.
    mac: [i32; 4],

    lzcs: u32,
    lzcr: u32,

    r23: u32,

    // r63
    flags: Flags,
}

bitfield! {
    pub struct Flags(u32);
    impl Debug;

    ir0_sat, set_ir0_sat: 12;
    ir1_sat, set_ir1_sat: 24;
    ir2_sat, set_ir2_sat: 23;
    ir3_sat, set_ir3_sat: 22;

    color_r_sat, set_color_r_sat: 21;
    color_g_sat, set_color_g_sat: 20;
    color_b_sat, set_color_b_sat: 19;

    mac0_of_pos, set_mac0_of_pos: 16;
    mac0_of_neg, set_mac0_of_neg: 15;
    mac1_of_pos, set_mac1_of_pos: 30;
    mac1_of_neg, set_mac1_of_neg: 27;
    mac2_of_pos, set_mac2_of_pos: 29;
    mac2_of_neg, set_mac2_of_neg: 26;
    mac3_of_pos, set_mac3_of_pos: 28;
    mac3_of_neg, set_mac3_of_neg: 25;

    sx2_sat, set_sx2_sat: 14;
    sy2_sat, set_sy2_sat: 13;

    sz3_otz_sat, set_sz3_otz_sat: 18;
    division_overflow, set_division_overflow: 17;

    error, set_error: 31;
}

impl Gte {
    pub fn hash(&self) -> u64 {
        // A simple hash function for the GTE state
        let mut hash: u64 = 0xcbf29ce484222325;
        for i in 0..64 {
            let val = self.read(i).unwrap();
            hash ^= val as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }

        hash
    }

    /// Creates a new GTE instance
    pub fn new() -> Self {
        Gte {
            current_instruction: 0,

            cr: [0; 32],

            rotation: [[0; 3]; 3],
            light: [[0; 3]; 3],
            color: [[0; 3]; 3],

            t: [0; 4],
            b: [0; 4],
            fc: [0; 4],
            null: [0; 4],

            ofx: 0,
            ofy: 0,
            h: 0,
            dqa: 0,
            dqb: 0,

            zsf3: 0,
            zsf4: 0,

            vectors: [[0; 4]; 3],
            rgb: RGB {
                r: 0,
                g: 0,
                b: 0,
                code: 0,
            },
            otz: 0,

            ir: [0; 4],

            xy_fifo: [XY { x: 0, y: 0 }; 4],
            z_fifo: [0; 4],
            rgb_fifo: [RGB {
                r: 0,
                g: 0,
                b: 0,
                code: 0,
            }; 3],
            mac: [0; 4],
            lzcs: 0,
            lzcr: 0,
            r23: 0,

            flags: Flags(0),
        }
    }

    /// Executes a GTE instruction
    pub fn execute(&mut self, instruction: Instruction) {
        let opcode = instruction.0 & 0x3f;

        // Flags are reset at the start of each instruction
        self.flags = Flags(0);

        self.current_instruction = instruction.0;

        match opcode {
            0x01 => self.ins_rtps(),
            0x06 => self.ins_nclip(),
            0x0c => self.ins_op(),
            0x10 => self.ins_dpcs(),
            0x11 => self.ins_intpl(),
            0x12 => self.ins_mvmva(),
            0x13 => self.ins_ncds(),
            0x14 => self.ins_cdp(),
            0x16 => self.ins_ncdt(),
            0x1b => self.ins_nccs(),
            0x1c => self.ins_cc(),
            0x1e => self.ins_ncs(),
            0x20 => self.ins_nct(),
            0x28 => self.ins_sqr(),
            0x29 => self.ins_dcpl(),
            0x2a => self.ins_dpct(),
            0x2d => self.ins_avsz3(),
            0x2e => self.ins_avsz4(),
            0x30 => self.ins_rtpt(),
            0x3d => self.ins_gpf(),
            0x3e => self.ins_gpl(),
            0x3f => self.ins_ncct(),
            _ => {
                // Unimplemented or invalid instruction
                panic!("Unimplemented GTE instruction: {:x}", opcode);
            }
        }
    }

    pub fn all_regs(&self) -> [u32; 64] {
        let mut regs = [0; 64];

        for i in 0..64 {
            regs[i] = self.read(i).unwrap_or(0);
        }

        regs
    }

    /// Writes a value to a GTE register
    pub fn write(&mut self, register: usize, value: u32) -> Result<(), String> {
        if register >= 32 {
            return self.write_cr(register - 32, value);
        }

        match register {
            0 => {
                self.vectors[0][0] = value as i16;
                self.vectors[0][1] = (value >> 16) as i16;
            }
            1 => {
                self.vectors[0][2] = value as i16;
            }
            2 => {
                self.vectors[1][0] = value as i16;
                self.vectors[1][1] = (value >> 16) as i16;
            }
            3 => {
                self.vectors[1][2] = value as i16;
            }
            4 => {
                self.vectors[2][0] = value as i16;
                self.vectors[2][1] = (value >> 16) as i16;
            }
            5 => {
                self.vectors[2][2] = value as i16;
            }
            6 => {
                self.rgb.r = value as u8;
                self.rgb.g = (value >> 8) as u8;
                self.rgb.b = (value >> 16) as u8;
                self.rgb.code = (value >> 24) as u8;
            }
            7 => {
                self.otz = value as u16;
            }
            8 => {
                self.ir[0] = value as i16;
            }
            9 => {
                self.ir[1] = value as i16;
            }
            10 => {
                self.ir[2] = value as i16;
            }
            11 => {
                self.ir[3] = value as i16;
            }
            12 => {
                self.xy_fifo[0].x = value as i16;
                self.xy_fifo[0].y = (value >> 16) as i16;
            }
            13 => {
                self.xy_fifo[1].x = value as i16;
                self.xy_fifo[1].y = (value >> 16) as i16;
            }
            14 => {
                self.xy_fifo[2].x = value as i16;
                self.xy_fifo[2].y = (value >> 16) as i16;
                self.xy_fifo[3].x = value as i16;
                self.xy_fifo[3].y = (value >> 16) as i16;
            }
            15 => {
                self.xy_fifo[3].x = value as i16;
                self.xy_fifo[3].y = (value >> 16) as i16;

                self.xy_fifo[0] = self.xy_fifo[1];
                self.xy_fifo[1] = self.xy_fifo[2];
                self.xy_fifo[2] = self.xy_fifo[3];
            }
            16 => {
                self.z_fifo[0] = value as u16;
            }
            17 => {
                self.z_fifo[1] = value as u16;
            }
            18 => {
                self.z_fifo[2] = value as u16;
            }
            19 => {
                self.z_fifo[3] = value as u16;
            }
            20 => {
                self.rgb_fifo[0].r = value as u8;
                self.rgb_fifo[0].g = (value >> 8) as u8;
                self.rgb_fifo[0].b = (value >> 16) as u8;
                self.rgb_fifo[0].code = (value >> 24) as u8;
            }
            21 => {
                self.rgb_fifo[1].r = value as u8;
                self.rgb_fifo[1].g = (value >> 8) as u8;
                self.rgb_fifo[1].b = (value >> 16) as u8;
                self.rgb_fifo[1].code = (value >> 24) as u8;
            }
            22 => {
                self.rgb_fifo[2].r = value as u8;
                self.rgb_fifo[2].g = (value >> 8) as u8;
                self.rgb_fifo[2].b = (value >> 16) as u8;
                self.rgb_fifo[2].code = (value >> 24) as u8;
            }
            23 => {
                self.r23 = value;
            }
            24 => {
                self.mac[0] = value as i32;
            }
            25 => {
                self.mac[1] = value as i32;
            }
            26 => {
                self.mac[2] = value as i32;
            }
            27 => {
                self.mac[3] = value as i32;
            }
            28 => {
                self.ir[1] = ((value & 0x1f) << 7) as i16;
                self.ir[2] = (((value >> 5) & 0x1f) << 7) as i16;
                self.ir[3] = (((value >> 10) & 0x1f) << 7) as i16;
            }
            29 => {}
            30 => {
                self.lzcs = value;
                self.lzcr = if self.lzcs as i32 >= 0 {
                    self.lzcs.leading_zeros()
                } else {
                    self.lzcs.leading_ones()
                }
            }
            31 => {}
            _ => return Err(format!("Invalid GTE register: {}", register)),
        }

        Ok(())
    }

    /// Reads a value from a GTE register
    pub fn read(&self, register: usize) -> Option<u32> {
        let val = match register {
            0 => {
                (self.vectors[0][0] as u16 as u32)
                    | ((self.vectors[0][1] as u16 as u32) << 16)
            }
            1 => self.vectors[0][2] as u32,
            2 => {
                (self.vectors[1][0] as u16 as u32)
                    | ((self.vectors[1][1] as u16 as u32) << 16)
            }
            3 => self.vectors[1][2] as u32,
            4 => {
                (self.vectors[2][0] as u16 as u32)
                    | ((self.vectors[2][1] as u16 as u32) << 16)
            }
            5 => self.vectors[2][2] as u32,
            6 => {
                self.rgb.r as u32
                    | ((self.rgb.g as u32) << 8)
                    | ((self.rgb.b as u32) << 16)
                    | ((self.rgb.code as u32) << 24)
            }
            7 => self.otz as u32,
            8 => self.ir[0] as u32,
            9 => self.ir[1] as u32,
            10 => self.ir[2] as u32,
            11 => self.ir[3] as u32,
            12 => {
                (self.xy_fifo[0].x as u16 as u32)
                    | ((self.xy_fifo[0].y as u16 as u32) << 16)
            }
            13 => {
                (self.xy_fifo[1].x as u16 as u32)
                    | ((self.xy_fifo[1].y as u16 as u32) << 16)
            }
            14 | 15 => {
                (self.xy_fifo[2].x as u16 as u32)
                    | ((self.xy_fifo[2].y as u16 as u32) << 16)
            }
            16 => self.z_fifo[0] as u32,
            17 => self.z_fifo[1] as u32,
            18 => self.z_fifo[2] as u32,
            19 => self.z_fifo[3] as u32,
            20 => {
                (self.rgb_fifo[0].r as u32)
                    | ((self.rgb_fifo[0].g as u32) << 8)
                    | ((self.rgb_fifo[0].b as u32) << 16)
                    | ((self.rgb_fifo[0].code as u32) << 24)
            }
            21 => {
                (self.rgb_fifo[1].r as u32)
                    | ((self.rgb_fifo[1].g as u32) << 8)
                    | ((self.rgb_fifo[1].b as u32) << 16)
                    | ((self.rgb_fifo[1].code as u32) << 24)
            }
            22 => {
                (self.rgb_fifo[2].r as u32)
                    | ((self.rgb_fifo[2].g as u32) << 8)
                    | ((self.rgb_fifo[2].b as u32) << 16)
                    | ((self.rgb_fifo[2].code as u32) << 24)
            }
            23 => self.r23,
            24 => self.mac[0] as u32,
            25 => self.mac[1] as u32,
            26 => self.mac[2] as u32,
            27 => self.mac[3] as u32,
            28 | 29 => {
                Gte::sat5(self.ir[1] >> 7) as u32
                    | ((Gte::sat5(self.ir[2] >> 7) as u32) << 5)
                    | ((Gte::sat5(self.ir[3] >> 7) as u32) << 10)
            }
            30 => self.lzcs,
            31 => {
                if self.lzcs as i32 >= 0 {
                    self.lzcs.leading_zeros()
                } else {
                    self.lzcs.leading_ones()
                }
            }

            56 => self.ofx as u32,
            57 => self.ofy as u32,
            58 => self.h as i16 as u32,
            59 => self.dqa as i16 as u32,
            60 => self.dqb as u32,
            61 => self.zsf3 as i16 as u32,
            62 => self.zsf4 as i16 as u32,
            63 => {
                let val = self.flags.0;
                if val & 0x7f87e000 != 0 {
                    val | (1 << 31)
                } else {
                    val
                }
            }
            36 | 44 | 52 => self.cr[register - 32] as i16 as u32,
            32..=63 => self.cr[register - 32],

            _ => return None,
        };

        Some(val)
    }

    pub fn write_cr(
        &mut self,
        register: usize,
        value: u32,
    ) -> Result<(), String> {
        const MASK_TABLE: [u32; 32] = [
            /* 0x00 */
            0xffff_ffff,
            0xffff_ffff,
            0xffff_ffff,
            0xffff_ffff,
            0x0000_ffff,
            0xffff_ffff,
            0xffff_ffff,
            0xffff_ffff,
            /* 8 */
            0xffff_ffff,
            0xffff_ffff,
            0xffff_ffff,
            0xffff_ffff,
            0x0000_ffff,
            0xffff_ffff,
            0xffff_ffff,
            0xffff_ffff,
            /* 16 */
            0xffff_ffff,
            0xffff_ffff,
            0xffff_ffff,
            0xffff_ffff,
            0x0000_ffff,
            0xffff_ffff,
            0xffff_ffff,
            0xffff_ffff,
            /* 24 */
            0xffff_ffff,
            0xffff_ffff,
            0x0000_ffff,
            0x0000_ffff,
            0xffff_ffff,
            0x0000_ffff,
            0x0000_ffff,
            0xffff_ffff,
        ];

        let value = value & MASK_TABLE[register];
        self.cr[register] = value | (self.cr[register] & !MASK_TABLE[register]);

        if register < 24 {
            let we = register >> 3;
            let index = register & 7;

            if index >= 5 {
                let vector = match we {
                    0 => &mut self.t,
                    1 => &mut self.b,
                    2 => &mut self.fc,
                    _ => unreachable!(),
                };

                vector[index - 5] = value as i32;
            } else {
                let matrix = match we {
                    0 => &mut self.rotation,
                    1 => &mut self.light,
                    2 => &mut self.color,
                    _ => unreachable!(),
                };

                match index {
                    0 => {
                        matrix[0][0] = value as i16;
                        matrix[0][1] = (value >> 16) as i16;
                    }
                    1 => {
                        matrix[0][2] = value as i16;
                        matrix[1][0] = (value >> 16) as i16;
                    }
                    2 => {
                        matrix[1][1] = value as i16;
                        matrix[1][2] = (value >> 16) as i16;
                    }
                    3 => {
                        matrix[2][0] = value as i16;
                        matrix[2][1] = (value >> 16) as i16;
                    }
                    4 => {
                        matrix[2][2] = value as i16;
                    }
                    _ => unreachable!(),
                }
            }

            return Ok(());
        }

        match register {
            24 => {
                self.ofx = value as i32;
            }
            25 => {
                self.ofy = value as i32;
            }
            26 => {
                self.h = value as u16;
            }
            27 => {
                self.dqa = value as i16;
            }
            28 => {
                self.dqb = value as i32;
            }
            29 => {
                self.zsf3 = value as i16;
            }
            30 => {
                self.zsf4 = value as i16;
            }
            31 => {
                self.flags.0 = value & 0x7fff_f000;
                if value & 0x7f87e000 != 0 {
                    self.flags.0 |= 1 << 31;
                }
            }
            _ => unreachable!(),
        }

        Ok(())
    }

    fn sat5(cc: i16) -> u8 {
        if cc < 0 {
            0
        } else if cc > 0x1f {
            0x1f
        } else {
            cc as u8
        }
    }
}
