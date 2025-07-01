use crate::cpu::gte::{
    Gte,
    types::{Accumulator, Matrix, Vector},
};

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

    pub(super) fn ins_nclip(&mut self) {
        self.mac0 = self.mac0_ovf(
            (self.xy_fifo[0].x as i64
                * (self.xy_fifo[1].y as i64 - self.xy_fifo[2].y as i64))
                + (self.xy_fifo[1].x as i64
                    * (self.xy_fifo[2].y as i64 - self.xy_fifo[0].y as i64))
                + (self.xy_fifo[2].x as i64
                    * (self.xy_fifo[0].y as i64 - self.xy_fifo[1].y as i64)),
        ) as i32;
    }

    pub(crate) fn ins_ncds(&mut self) {
        self.norm_color_depth_cue(0, self.sf(), self.lm());
    }

    pub(crate) fn ins_ncdt(&mut self) {
        for i in 0..3 {
            self.norm_color_depth_cue(i, self.sf(), self.lm());
        }
    }

    pub(crate) fn ins_sqr(&mut self) {
        self.mac[0] = (self.ir[0] as i32 * self.ir[0] as i32) >> self.sf();
        self.mac[1] = (self.ir[1] as i32 * self.ir[1] as i32) >> self.sf();
        self.mac[2] = (self.ir[2] as i32 * self.ir[2] as i32) >> self.sf();

        self.mac_to_ir(self.lm());
    }

    pub(crate) fn ins_ncs(&mut self) {
        self.norm_color(self.sf(), self.lm(), 0);
    }

    pub(crate) fn ins_nct(&mut self) {
        for i in 0..3 {
            self.norm_color(self.sf(), self.lm(), i);
        }
    }

    pub(crate) fn ins_dcpl(&mut self) {
        self.depth_cue(true, false, self.sf(), self.lm());
    }

    pub(crate) fn ins_dpcs(&mut self) {
        self.depth_cue(false, false, self.sf(), self.lm());
    }

    pub(crate) fn ins_dpct(&mut self) {
        for _ in 0..3 {
            self.depth_cue(false, true, self.sf(), self.lm());
        }
    }

    pub(super) fn ins_avsz4(&mut self) {
        self.mac0 = self.f(self.zsf4 as i64
            * (self.z_fifo[0] as i64
                + self.z_fifo[1] as i64
                + self.z_fifo[2] as i64
                + self.z_fifo[3] as i64)) as i32;
        self.otz = self.lm_d(self.mac0 >> 12, false) as u16;
    }

    fn norm_color(&mut self, sf: u32, lm: bool, v: usize) {
        self.multiply_matrix_by_vector(
            self.light,
            self.vectors[v],
            self.null,
            sf,
            lm,
            false,
        );

        self.multiply_matrix_by_vector(
            self.color, self.ir, self.b, sf, lm, false,
        );

        self.mac_to_rgb_fifo();
    }

    pub(crate) fn ins_cc(&mut self) {
        self.multiply_matrix_by_vector(
            self.color,
            self.ir,
            self.b,
            self.sf(),
            self.lm(),
            false,
        );

        self.mac[0] =
            (((self.rgb.r as i32) << 4) * self.ir[0] as i32) >> self.sf();
        self.mac[1] =
            (((self.rgb.g as i32) << 4) * self.ir[1] as i32) >> self.sf();
        self.mac[2] =
            (((self.rgb.b as i32) << 4) * self.ir[2] as i32) >> self.sf();

        self.mac_to_ir(self.lm());
        self.mac_to_rgb_fifo();
    }

    pub(crate) fn ins_nccs(&mut self) {
        self.norm_color_color(0, self.sf(), self.lm());
    }

    pub(super) fn ins_rtpt(&mut self) {
        for i in 0..3 {
            self.multiply_matrix_by_vector_pt(
                self.rotation,
                self.vectors[i],
                self.t,
                self.sf(),
                self.lm(),
            );
            let (h_div_sz, of) =
                super::division::division(self.h, self.z_fifo[3]);
            let h_div_sz = h_div_sz as i64;

            if of {
                self.flags.set_division_overflow(true);
            }

            self.transform_xy(h_div_sz);

            if i == 2 {
                self.transform_dq(h_div_sz);
            }
        }
    }

    fn norm_color_depth_cue(&mut self, v: usize, sf: u32, lm: bool) {
        self.multiply_matrix_by_vector(
            self.light,
            self.vectors[v],
            self.null,
            sf,
            lm,
            false,
        );

        self.multiply_matrix_by_vector(
            self.color, self.ir, self.b, sf, lm, false,
        );

        self.depth_cue(true, false, sf, lm);
    }

    fn depth_cue(
        &mut self,
        mult_ir123: bool,
        rgb_from_fifo: bool,
        sf: u32,
        lm: bool,
    ) {
        let mut rgb_temp: [i32; 3] = [0; 3];
        let ir_temp: [i32; 3] =
            [self.ir[0] as i32, self.ir[1] as i32, self.ir[2] as i32];

        if rgb_from_fifo {
            rgb_temp[0] = (self.rgb_fifo[0].r as i32) << 4;
            rgb_temp[1] = (self.rgb_fifo[0].g as i32) << 4;
            rgb_temp[2] = (self.rgb_fifo[0].b as i32) << 4;
        } else {
            rgb_temp[0] = (self.rgb.r as i32) << 4;
            rgb_temp[1] = (self.rgb.g as i32) << 4;
            rgb_temp[2] = (self.rgb.b as i32) << 4;
        }

        if mult_ir123 {
            for i in 0..3 {
                self.mac[i] = (self.a_mv(
                    i,
                    ((self.fc[i] as i64) << 12)
                        - rgb_temp[i] as i64 * ir_temp[i] as i64,
                ) >> sf) as i32;
                let lm_b = self.lm_b(i, self.mac[i], false) as i64;
                self.mac[i] = (self.a_mv(
                    i,
                    rgb_temp[i] as i64 * ir_temp[i] as i64
                        + self.ir0 as i64 * lm_b,
                ) >> sf) as i32;
            }
        } else {
            for i in 0..3 {
                self.mac[i] = (self.a_mv(
                    i,
                    ((self.fc[i] as i64) << 12)
                        - ((rgb_temp[i] as u32) << 12) as i64,
                ) >> sf) as i32;
                let lm_b = self.lm_b(i, self.mac[i], false) as i64;
                self.mac[i] = (self.a_mv(
                    i,
                    ((rgb_temp[i] as i64) << 12) + self.ir0 as i64 * lm_b,
                ) >> sf) as i32;
            }
        }

        self.mac_to_ir(lm);
        self.mac_to_rgb_fifo();
    }

    fn mac_to_rgb_fifo(&mut self) {
        self.rgb_fifo[0] = self.rgb_fifo[1];
        self.rgb_fifo[1] = self.rgb_fifo[2];
        self.rgb_fifo[2].r = self.clamp_color(0, self.mac[0] >> 4);
        self.rgb_fifo[2].g = self.clamp_color(1, self.mac[1] >> 4);
        self.rgb_fifo[2].b = self.clamp_color(2, self.mac[2] >> 4);
        self.rgb_fifo[2].code = self.rgb.code;
    }

    fn multiply_matrix_by_vector_pt(
        &mut self,
        matrix: Matrix,
        v: Vector<i16>,
        crv: Vector<i32>,
        sf: u32,
        lm: bool,
    ) {
        let mut tmp: [i64; 3] = [0; 3];

        for i in 0..3 {
            let mulr = matrix.row(i) * v;

            tmp[i] = (crv[i] as i64) << 12;

            tmp[i] = self.a_mv(i, tmp[i] + mulr[0] as i64);
            tmp[i] = self.a_mv(i, tmp[i] + mulr[1] as i64);
            tmp[i] = self.a_mv(i, tmp[i] + mulr[2] as i64);

            self.mac[i] = (tmp[i] >> sf) as i32;
        }

        self.ir[0] = self.lm_b(0, self.mac[0], lm);
        self.ir[1] = self.lm_b(1, self.mac[1], lm);
        self.ir[2] = self.lm_b_ptz(2, self.mac[2], (tmp[2] >> 12) as i32, lm);

        self.z_fifo[0] = self.z_fifo[1];
        self.z_fifo[1] = self.z_fifo[2];
        self.z_fifo[2] = self.z_fifo[3];
        self.z_fifo[3] = self.lm_d((tmp[2] >> 12) as i32, true) as u16;
    }

    fn multiply_matrix_by_vector(
        &mut self,
        matrix: Matrix,
        v: Vector<i16>,
        crv: Vector<i32>,
        sf: u32,
        lm: bool,
        is_fc: bool,
    ) {
        for i in 0..3 {
            let mut acc = Accumulator::new();
            acc.set((crv[i] as i64) << 12);

            let mulr = matrix.row(i) * v;

            acc += mulr[0];
            if is_fc {
                self.lm_b(i, acc.commit(sf), false);
                acc.set(0);
            }
            acc += mulr[1];
            acc += mulr[2];

            if acc.positive_overflow {
                self.flags.0 |= 1 << (30 - i);
            }
            if acc.negative_overflow {
                self.flags.0 |= 1 << (27 - i);
            }
            self.mac[i] = acc.commit(sf);
        }

        self.mac_to_ir(lm);
    }

    fn mac0_ovf(&mut self, value: i64) -> i64 {
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

    pub(crate) fn ins_intpl(&mut self) {
        let irs: [i32; 3] = [
            (self.ir[0] as i32) << 12,
            (self.ir[1] as i32) << 12,
            (self.ir[2] as i32) << 12,
        ];

        let ir1 = (((self.fc[0] as i64) << 12) - irs[0] as i64) >> self.sf();
        let ir2 = (((self.fc[1] as i64) << 12) - irs[1] as i64) >> self.sf();
        let ir3 = (((self.fc[2] as i64) << 12) - irs[2] as i64) >> self.sf();

        self.ir[0] = self.lm_b(1, ir1 as i32, false);
        self.ir[1] = self.lm_b(2, ir2 as i32, false);
        self.ir[2] = self.lm_b(3, ir3 as i32, false);

        let ir1 = ((self.ir[0] as i64 * self.ir0 as i64) + irs[0] as i64)
            >> self.sf();
        let ir2 = ((self.ir[1] as i64 * self.ir0 as i64) + irs[1] as i64)
            >> self.sf();
        let ir3 = ((self.ir[2] as i64 * self.ir0 as i64) + irs[2] as i64)
            >> self.sf();

        self.mac[0] = self.lm_b(1, ir1 as i32, self.lm()) as i32;
        self.mac[1] = self.lm_b(2, ir2 as i32, self.lm()) as i32;
        self.mac[2] = self.lm_b(3, ir3 as i32, self.lm()) as i32;

        self.ir[0] = self.lm_b(1, self.mac[0], self.lm());
        self.ir[1] = self.lm_b(2, self.mac[1], self.lm());
        self.ir[2] = self.lm_b(3, self.mac[2], self.lm());

        self.mac_to_rgb_fifo();
    }

    fn clamp_color(&mut self, which: usize, value: i32) -> u8 {
        if value > 0xff {
            self.flags.0 |= 1 << (21 - which);
            0xff
        } else if value < 0 {
            self.flags.0 |= 1 << (21 - which);
            0
        } else {
            value as u8
        }
    }

    fn transform_xy(&mut self, h_div_sz: i64) {
        self.mac0 = (self.f(self.ofx as i64 + self.ir[0] as i64 * h_div_sz)
            >> 16) as i32;
        self.xy_fifo[3].x = self.lm_g(0, self.mac0) as i16;

        self.mac0 = (self.f(self.ofy as i64 + self.ir[1] as i64 * h_div_sz)
            >> 16) as i32;
        self.xy_fifo[3].y = self.lm_g(1, self.mac0) as i16;

        self.xy_fifo[0] = self.xy_fifo[1];
        self.xy_fifo[1] = self.xy_fifo[2];
        self.xy_fifo[2] = self.xy_fifo[3];
    }

    fn transform_dq(&mut self, h_div_sz: i64) {
        self.mac0 = self.f(self.dqb as i64 + self.dqa as i64 * h_div_sz) as i32;
        self.ir0 = self
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

    pub(super) fn ins_avsz3(&mut self) {
        self.mac0 = self.mac0_ovf(
            self.zsf3 as i64
                * (self.z_fifo[1] as i64
                    + self.z_fifo[2] as i64
                    + self.z_fifo[3] as i64),
        ) as i32;
        self.otz = self.lm_d(self.mac0 >> 12, false) as u16;
    }

    pub(crate) fn ins_mvmva(&mut self) {
        let matrix = match self.mx() {
            0 => self.rotation,
            1 => self.light,
            2 => self.color,
            3 => Matrix([
                Vector([
                    -(self.rgb.r as i16) << 4,
                    (self.rgb.r as i16) << 4,
                    self.ir0,
                ]),
                Vector([
                    self.cr[1] as i16,
                    self.cr[1] as i16,
                    self.cr[1] as i16,
                ]),
                Vector([
                    self.cr[2] as i16,
                    self.cr[2] as i16,
                    self.cr[2] as i16,
                ]),
            ]),
            _ => unreachable!(),
        };

        self.multiply_matrix_by_vector(
            matrix,
            self.v(),
            self.cv(),
            self.sf(),
            self.lm(),
            (self.current_instruction >> 13) & 3 == 2,
        );
    }

    fn mac_to_ir(&mut self, lm: bool) {
        self.ir[0] = self.lm_b(0, self.mac[0], lm);
        self.ir[1] = self.lm_b(1, self.mac[1], lm);
        self.ir[2] = self.lm_b(2, self.mac[2], lm);
    }

    fn v(&self) -> Vector<i16> {
        if self.v_i() == 3 {
            self.ir
        } else {
            self.vectors[self.v_i()]
        }
    }

    fn cv(&self) -> Vector<i32> {
        match (self.current_instruction >> 13) & 3 {
            0 => self.t,
            1 => self.b,
            2 => self.fc,
            3 => self.null,
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

    pub(crate) fn ins_gpf(&mut self) {
        self.mac[0] = (self.ir0 as i32 * self.ir[0] as i32) >> self.sf();
        self.mac[1] = (self.ir0 as i32 * self.ir[1] as i32) >> self.sf();
        self.mac[2] = (self.ir0 as i32 * self.ir[2] as i32) >> self.sf();

        self.mac_to_ir(self.lm());
        self.mac_to_rgb_fifo();
    }

    pub(crate) fn ins_gpl(&mut self) {
        let ir = self.ir * self.ir0;

        self.mac[0] = self.a_mv(
            0,
            (((self.mac[0] as i64) << self.sf()) + (ir[0] as i64)) >> self.sf(),
        ) as i32;
        self.mac[1] = self.a_mv(
            1,
            (((self.mac[1] as i64) << self.sf()) + (ir[1] as i64)) >> self.sf(),
        ) as i32;
        self.mac[2] = self.a_mv(
            2,
            (((self.mac[2] as i64) << self.sf()) + (ir[2] as i64)) >> self.sf(),
        ) as i32;

        self.mac_to_ir(self.lm());
        self.mac_to_rgb_fifo();
    }

    pub(crate) fn ins_ncct(&mut self) {
        for i in 0..3 {
            self.norm_color_color(i, self.sf(), self.lm());
        }
    }

    fn norm_color_color(&mut self, v: usize, sf: u32, lm: bool) {
        self.multiply_matrix_by_vector(
            self.light,
            self.vectors[v],
            self.null,
            sf,
            lm,
            false,
        );

        self.multiply_matrix_by_vector(
            self.color, self.ir, self.b, sf, lm, false,
        );

        self.mac[0] = (((self.rgb.r as i32) << 4) * self.ir[0] as i32) >> sf;
        self.mac[1] = (((self.rgb.g as i32) << 4) * self.ir[1] as i32) >> sf;
        self.mac[2] = (((self.rgb.b as i32) << 4) * self.ir[2] as i32) >> sf;

        self.mac_to_ir(lm);

        self.mac_to_rgb_fifo();
    }

    pub(super) fn ins_op(&mut self) {
        self.mac[0] = ((self.rotation.0[1][1] as i32 * self.ir[2] as i32)
            - (self.rotation.0[2][2] as i32 * self.ir[1] as i32))
            >> self.sf();
        self.mac[1] = ((self.rotation.0[2][2] as i32 * self.ir[0] as i32)
            - (self.rotation.0[0][0] as i32 * self.ir[2] as i32))
            >> self.sf();
        self.mac[2] = ((self.rotation.0[0][0] as i32 * self.ir[1] as i32)
            - (self.rotation.0[1][1] as i32 * self.ir[0] as i32))
            >> self.sf();

        self.mac_to_ir(self.lm());
    }

    pub(crate) fn ins_cdp(&mut self) {
        self.multiply_matrix_by_vector(
            self.color,
            self.ir,
            self.b,
            self.sf(),
            self.lm(),
            false,
        );

        self.depth_cue(true, false, self.sf(), self.lm());
    }
}
