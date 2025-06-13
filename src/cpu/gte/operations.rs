use crate::cpu::gte::{Gte, Matrix};

macro_rules! sign_x_to_s64 {
    ($n:expr, $val:expr) => {
        ((($val) as u64 as i64) << (64 - ($n))) >> (64 - ($n))
    };
}

impl Gte {
    fn sf(&self) -> u32 {
        if self.current_instruction & (1 << 19) != 0 {
            12
        } else {
            0
        }
    }

    fn lm(&self) -> bool {
        (self.current_instruction >> 10 & 1) != 0
    }

    fn mx(&self) -> u32 {
        (self.current_instruction >> 17) & 0x3
    }

    fn v_i(&self) -> usize {
        ((self.current_instruction >> 15) & 0x3) as usize
    }

    pub(super) fn ins_rtps(&mut self) {
        self.multiply_matrix_by_vector_pt(
            self.rotation,
            self.vectors[0],
            self.t,
            self.sf(),
            self.lm(),
        );
        let (h_div_sz, of) = super::division::division(self.h, self.z_fifo[3]);
        let h_div_sz = h_div_sz as i64;

        if of {
            self.flags.set_division_overflow(true);
        }

        self.transform_xy(h_div_sz);
        self.transform_dq(h_div_sz);
    }

    fn multiply_matrix_by_vector_pt(
        &mut self,
        matrix: Matrix,
        v: [i16; 4],
        crv: [i32; 4],
        sf: u32,
        lm: bool,
    ) {
        let mut tmp: [i64; 3] = [0; 3];

        for i in 0..3 {
            let mut mulr: [i32; 3] = [0; 3];

            tmp[i] = (crv[i] as i64) << 12;

            mulr[0] = matrix[i][0] as i32 * v[0] as i32;
            mulr[1] = matrix[i][1] as i32 * v[1] as i32;
            mulr[2] = matrix[i][2] as i32 * v[2] as i32;

            tmp[i] = self.a_mv(i, tmp[i] + mulr[0] as i64);
            tmp[i] = self.a_mv(i, tmp[i] + mulr[1] as i64);
            tmp[i] = self.a_mv(i, tmp[i] + mulr[2] as i64);

            self.mac[1 + i] = (tmp[i] >> sf) as i32;
        }

        self.ir[1] = self.lm_b(0, self.mac[1], lm);
        self.ir[2] = self.lm_b(1, self.mac[2], lm);
        self.ir[3] = self.lm_b_ptz(2, self.mac[3], (tmp[2] >> 12) as i32, lm);

        self.z_fifo[0] = self.z_fifo[1];
        self.z_fifo[1] = self.z_fifo[2];
        self.z_fifo[2] = self.z_fifo[3];
        self.z_fifo[3] = self.lm_d((tmp[2] >> 12) as i32, true) as u16;
    }

    fn a_mv(&mut self, which: usize, value: i64) -> i64 {
        if value >= (1_i64 << 43) {
            self.flags.0 |= 1 << (30 - which);
        }

        if value < -(1_i64 << 43) {
            self.flags.0 |= 1 << (27 - which);
        }

        sign_x_to_s64!(44, value)
    }

    fn lm_d(&mut self, value: i32, unchained: bool) -> i32 {
        // Not sure if we should have it as int64, or just chain on to and special case
        // when the F flags are set.
        if !unchained {
            if self.flags.mac0_of_neg() {
                self.flags.set_sz3_otz_sat(true);
                return 0;
            }

            if self.flags.mac0_of_pos() {
                self.flags.set_sz3_otz_sat(true);
                return 0xffff;
            }
        }

        if value < 0 {
            self.flags.set_sz3_otz_sat(true);
            0
        } else if value > 0xffff {
            self.flags.set_sz3_otz_sat(true);
            0xffff
        } else {
            value
        }
    }

    fn transform_xy(&mut self, h_div_sz: i64) {
        self.mac[0] = (self.f(self.ofx as i64 + self.ir[1] as i64 * h_div_sz)
            >> 16) as i32;
        self.xy_fifo[3].x = self.lm_g(0, self.mac[0]) as i16;

        self.mac[0] = (self.f(self.ofy as i64 + self.ir[2] as i64 * h_div_sz)
            >> 16) as i32;
        self.xy_fifo[3].y = self.lm_g(1, self.mac[0]) as i16;

        self.xy_fifo[0] = self.xy_fifo[1];
        self.xy_fifo[1] = self.xy_fifo[2];
        self.xy_fifo[2] = self.xy_fifo[3];
    }

    fn transform_dq(&mut self, h_div_sz: i64) {
        self.mac[0] =
            self.f(self.dqb as i64 + self.dqa as i64 * h_div_sz) as i32;
        self.ir[0] = self
            .lm_h(((self.dqb as i64 + self.dqa as i64 * h_div_sz) >> 12) as i32)
            as i16;
    }

    fn f(&mut self, value: i64) -> i64 {
        if value < -0x8000_0000 {
            // flag set here
            self.flags.set_mac0_of_neg(true);
        }

        if value > 0x7fff_ffff {
            // flag set here
            self.flags.set_mac0_of_pos(true);
        }

        value
    }

    fn lm_h(&mut self, value: i32) -> i16 {
        if value < 0 {
            self.flags.set_ir0_sat(true);
            0
        } else if value > 0x1000 {
            self.flags.set_ir0_sat(true);
            0x1000
        } else {
            value as i16
        }
    }

    fn lm_b(&mut self, which: usize, value: i32, lm: bool) -> i16 {
        let min: i32 = if lm { 0 } else { -0x8000 };
        if value < min {
            self.flags.0 |= 1 << (24 - which);
            min as i16
        } else if value > 0x7fff {
            self.flags.0 |= 1 << (24 - which);
            0x7fff
        } else {
            value as i16
        }
    }

    fn lm_b_ptz(
        &mut self,
        which: usize,
        value: i32,
        ftv_value: i32,
        lm: bool,
    ) -> i16 {
        let tmp: i32 = if lm { 0x8000 } else { 0 };

        if ftv_value < -0x8000 {
            self.flags.0 |= 1 << (24 - which);
        }

        if ftv_value > 0x7fff {
            self.flags.0 |= 1 << (24 - which);
        }

        if value < (-0x8000 + tmp) {
            (-0x8000 + tmp) as i16
        } else if value > 0x7fff {
            0x7fff
        } else {
            value as i16
        }
    }

    pub(crate) fn ins_mvmva(&mut self) {
        let matrix = match self.mx() {
            0 => self.rotation,
            1 => self.light,
            2 => self.color,
            3 => {
                // warn!(self.logger, "Use of bogus matrix in mvmva");

                [
                    [
                        -(self.rgb.r as i16) << 4,
                        (self.rgb.r as i16) << 4,
                        self.ir[0],
                    ],
                    [self.cr[1] as i16, self.cr[1] as i16, self.cr[1] as i16],
                    [self.cr[2] as i16, self.cr[2] as i16, self.cr[2] as i16],
                ]
            }
            _ => unreachable!(),
        };
        self.multiply_matrix_by_vector(
            matrix,
            self.v(),
            *self.cv(),
            self.sf(),
            self.lm(),
        );
    }

    fn multiply_matrix_by_vector(
        &mut self,
        matrix: Matrix,
        v: [i16; 4],
        crv: [i32; 4],
        sf: u32,
        lm: bool,
    ) {
        for i in 0..3 {
            let mut mulr: [i32; 3] = [0; 3];

            let mut tmp = (crv[i] as i64) << 12;

            mulr[0] = matrix[i][0] as i32 * v[0] as i32;
            mulr[1] = matrix[i][1] as i32 * v[1] as i32;
            mulr[2] = matrix[i][2] as i32 * v[2] as i32;

            tmp = self.a_mv(i, tmp + mulr[0] as i64);
            // TODO: this should be a ref
            if crv == self.fc {
                self.lm_b(i, (tmp >> sf) as i32, false);
                tmp = 0;
            }
            tmp = self.a_mv(i, tmp + mulr[1] as i64);
            tmp = self.a_mv(i, tmp + mulr[2] as i64);

            self.mac[1 + i] = (tmp >> sf) as i32;
        }

        self.mac_to_ir(lm);
    }

    fn mac_to_ir(&mut self, lm: bool) {
        self.ir[1] = self.lm_b(0, self.mac[1], lm);
        self.ir[2] = self.lm_b(1, self.mac[2], lm);
        self.ir[3] = self.lm_b(2, self.mac[3], lm);
    }

    fn v(&self) -> [i16; 4] {
        if self.v_i() == 3 {
            [self.ir[1], self.ir[2], self.ir[3], 0]
        } else {
            [
                self.vectors[self.v_i()][0],
                self.vectors[self.v_i()][1],
                self.vectors[self.v_i()][2],
                0,
            ]
        }
    }

    fn cv(&self) -> &[i32; 4] {
        match (self.current_instruction >> 13) & 3 {
            0 => &self.t,
            1 => &self.b,
            2 => &self.fc,
            3 => &self.null,
            _ => unreachable!(),
        }
    }

    fn lm_g(&mut self, which: usize, value: i32) -> i16 {
        if value < -0x400 {
            self.flags.0 |= 1 << (14 - which);
            -0x400
        } else if value > 0x3ff {
            self.flags.0 |= 1 << (14 - which);
            0x3ff
        } else {
            value as i16
        }
    }
}
