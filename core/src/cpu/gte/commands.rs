use tracing::debug;

use super::*;

impl GTEngine {
    /// Perspective transformation(single)
    pub fn rtps(&mut self) {
        debug!("gte command, rtps");
    }

    /// Normal clipping
    pub fn nclip(&mut self) {
        debug!("gte command, nclip");

        let [x0, y0] = self.sxy[0];
        let [x1, y1] = self.sxy[1];
        let [x2, y2] = self.sxy[2];

        let (x0, y0) = (x0 as i64, y0 as i64);
        let (x1, y1) = (x1 as i64, y1 as i64);
        let (x2, y2) = (x2 as i64, y2 as i64);

        let a = x0 * (y1 - y2);
        let b = x1 * (y2 - y0);
        let c = x2 * (y0 - y1);

        self.mac[0] = self.i64_to_i32(a + b + c);
    }

    /// Cross product of two vectors
    pub fn op(&mut self, fields: CommandFields) {
        debug!("gte command, op");

        let [_, ir1, ir2, ir3] = self.ir;
        let (ir1, ir2, ir3) = (ir1 as i32, ir2 as i32, ir3 as i32);

        let sf = fields.sf() * 12;

        let rtm = &self.matrices[Matrix::Rotation as usize];

        let d1 = rtm[0][0] as i32;
        let d2 = rtm[1][1] as i32;
        let d3 = rtm[2][2] as i32;

        self.mac[1] = (d2 * ir3 - d3 * ir2) >> sf;
        self.mac[2] = (d3 * ir1 - d1 * ir3) >> sf;
        self.mac[3] = (d1 * ir2 - d2 * ir1) >> sf;

        self.mac_to_ir(fields);
    }

    /// Depth cueing (single)
    pub fn dpcs(&mut self, fields: CommandFields) {
        debug!("gte command, dpcs");

        let sf = fields.sf() * 12;
        let far_colors = self.control_vectors[ControlVec::FarColor as usize];

        far_colors.iter().enumerate().for_each(|(i, &fc)| {
            let c = (self.rgbc[i] as i64) << 16;
            let fc = (fc as i64) << 12;
            let ir0 = self.ir[0] as i64;

            let sub = (self.i64_to_i44(i + 1, fc - c) >> sf) as i32;
            let sat = self.i32_to_i16(i + 1, sub, Saturation::S16) as i64;
            let res = self.i64_to_i44(i + 1, c + ir0 * sat);

            self.mac[i + 1] = (res >> sf) as i32;
        });

        self.mac_to_ir(fields);
        self.mac_to_color_push();
    }

    /// Interpolation of a vector and far color
    pub fn intpl(&mut self, fields: CommandFields) {
        debug!("gte command, intpl");

        let sf = fields.sf() * 12;
        let far_colors = self.control_vectors[ControlVec::FarColor as usize];

        far_colors.iter().enumerate().for_each(|(i, &fc)| {
            let ir = (self.ir[i + 1] as i64) << 12;
            let fc = (fc as i64) << 12;
            let ir0 = self.ir[0] as i64;

            let sub = (self.i64_to_i44(i + 1, fc - ir) >> sf) as i32;
            let sat = self.i32_to_i16(i + 1, sub, Saturation::S16) as i64;
            let res = self.i64_to_i44(i + 1, ir + ir0 * sat);

            self.mac[i + 1] = (res >> sf) as i32;
        });

        self.mac_to_ir(fields);
        self.mac_to_color_push();
    }

    /// Multiply vector by matrix and vector addition
    pub fn mvmva(&mut self, fields: CommandFields) {
        debug!("gte command, mvmva");
        self.multiply_matrix_by_vector(fields, fields.mx(), fields.vx(), fields.cv());
    }

    /// Normal color depth cue single vector
    pub fn ncds(&mut self, fields: CommandFields) {
        debug!("gte command, ncds");

        self.multiply_matrix_by_vector(fields, Matrix::Light, Vector::V0, ControlVec::None);
        self.multiply_matrix_by_vector(fields, Matrix::Color, Vector::IR, ControlVec::Background);

        self.dcpl(fields);
    }

    ///
    pub fn cdp(&mut self) {
        debug!("gte command, cdp");
    }

    ///
    pub fn ncdt(&mut self) {
        debug!("gte command, ncdt");
    }

    ///
    pub fn nccs(&mut self) {
        debug!("gte command, nccs");
    }

    ///
    pub fn cc(&mut self) {
        debug!("gte command, cc");
    }

    ///
    pub fn ncs(&mut self) {
        debug!("gte command, ncs");
    }

    ///
    pub fn nct(&mut self) {
        debug!("gte command, nct");
    }

    ///
    pub fn sqr(&mut self) {
        debug!("gte command, sqr");
    }

    /// Depth cue color light
    pub fn dcpl(&mut self, fields: CommandFields) {
        debug!("gte command, dcpl");

        let sf = fields.sf() * 12;
        let far_colors = self.control_vectors[ControlVec::FarColor as usize];

        far_colors.iter().enumerate().for_each(|(i, &fc)| {
            let col = self.rgbc[i] as i64;
            let ir = self.ir[i + 1] as i64;
            let fc = (fc as i64) << 12;

            let shaded = (col * ir) << 4;
            let ir0 = self.ir[0] as i64;

            let sub = (self.i64_to_i44(i + 1, fc - shaded) >> sf) as i32;

            let sat = self.i32_to_i16(i + 1, sub, Saturation::S16) as i64;

            let res = self.i64_to_i44(i + 1, shaded + ir0 * sat);

            self.mac[i + 1] = (res >> sf) as i32;
        });

        self.mac_to_ir(fields);
        self.mac_to_color_push();
    }

    /// Depth cueing (triple)
    pub fn dpct(&mut self) {
        debug!("gte command, dpct");
    }

    ///
    pub fn avsz3(&mut self) {
        debug!("gte command, avsz3");
    }

    ///
    pub fn avsz4(&mut self) {
        debug!("gte command, avsz4");
    }

    /// Perspective transformation(triple)
    pub fn rtpt(&mut self) {
        debug!("gte command, rtpt");
    }

    ///
    pub fn gpf(&mut self) {
        debug!("gte command, gpf");
    }

    ///
    pub fn gpl(&mut self) {
        debug!("gte command, gpl");
    }

    ///
    pub fn ncct(&mut self) {
        debug!("gte command, ncct");
    }

    // --------------------------Helper Flag and Math Functions------------------------- //

    fn i64_to_i32(&mut self, val: i64) -> i32 {
        if val > i32::MAX.into() {
            self.flag.mac0_overflow_pos(true);
        } else if val < i32::MIN.into() {
            self.flag.mac0_overflow_neg(true);
        }

        ((val << 32) >> 32) as i32
    }

    fn i64_to_i44(&mut self, flag: usize, val: i64) -> i64 {
        debug_assert!(matches!(flag, 1..=3));

        if val > 0x7FF_FFFF_FFFF {
            match flag {
                1 => self.flag.mac1_overflow_pos(true),
                2 => self.flag.mac2_overflow_pos(true),
                3 => self.flag.mac3_overflow_pos(true),
                _ => unreachable!(),
            }
        } else if val < -0x800_0000_0000 {
            match flag {
                1 => self.flag.mac1_overflow_neg(true),
                2 => self.flag.mac2_overflow_neg(true),
                3 => self.flag.mac3_overflow_neg(true),
                _ => unreachable!(),
            }
        }

        (val << 20) >> 20
    }

    fn i32_to_i16(&mut self, flag: usize, val: i32, lm: Saturation) -> i16 {
        debug_assert!(matches!(flag, 1..=3));

        let min = match lm {
            Saturation::S16 => i16::MIN.into(),
            Saturation::U15 => 0,
        };

        let max = i16::MAX.into();

        let (res, saturated) = if val > max {
            (max as i16, true)
        } else if val < min {
            (min as i16, true)
        } else {
            (val as i16, false)
        };

        if saturated {
            match flag {
                1 => self.flag.ir1_saturated(true),
                2 => self.flag.ir2_saturated(true),
                3 => self.flag.ir3_saturated(true),
                _ => unreachable!(),
            }
        }

        res
    }

    fn i32_to_u8(&mut self, flag: usize, val: i32) -> u8 {
        debug_assert!(matches!(flag, 0..3));

        let (min, max) = (u8::MIN.into(), u8::MAX.into());

        let (res, saturated) = if val > max {
            (max as u8, true)
        } else if val < min {
            (min as u8, true)
        } else {
            (val as u8, false)
        };

        if saturated {
            match flag {
                0 => self.flag.cfifo_r_saturated(true),
                1 => self.flag.cfifo_g_saturated(true),
                2 => self.flag.cfifo_b_saturated(true),
                _ => unreachable!(),
            }
        }

        res
    }

    fn multiply_matrix_by_vector(
        &mut self,
        fields: CommandFields,
        mx: Matrix,
        vx: Vector,
        cv: ControlVec,
    ) {
        if mx == Matrix::Reserved {
            // Filling last matrix with garbage
            let r13 = self.matrices[0][0][2];
            let r22 = self.matrices[0][1][1];

            self.matrices[3] = [
                [
                    -(self.rgbc[0] as i16) << 4,
                    (self.rgbc[0] as i16) << 4,
                    self.ir[0],
                ],
                [r13, r13, r13],
                [r22, r22, r22],
            ];
        }

        // Last vector is IR vector
        if vx == Vector::IR {
            self.v[3] = [self.ir[1], self.ir[2], self.ir[3]];
        }

        let is_farcolor = cv == ControlVec::FarColor;

        let mx = mx as usize;
        let cv = cv as usize;
        let vx = vx as usize;

        let sf = fields.sf() * 12;
        let lm = fields.lm();

        for r in 0..3 {
            let prod1 = (self.v[vx][0] as i32) * (self.matrices[mx][r][0] as i32);
            let prod2 = (self.v[vx][1] as i32) * (self.matrices[mx][r][1] as i32);
            let prod3 = (self.v[vx][2] as i32) * (self.matrices[mx][r][2] as i32);

            let mut res;
            let add = (self.control_vectors[cv][r] as i64) << 12;

            // Bugged
            if is_farcolor {
                res = self.i64_to_i44(r + 1, prod2 as i64);
                res = self.i64_to_i44(r + 1, res + prod3 as i64);

                // Calculation is ignored but flag is still set, wtf!
                let sum = self.i64_to_i44(r + 1, add + prod1 as i64);
                let mac = (sum >> sf) as i32;
                let _ = self.i32_to_i16(r + 1, mac, lm);
            } else {
                res = add;
                res = self.i64_to_i44(r + 1, res + prod1 as i64);
                res = self.i64_to_i44(r + 1, res + prod2 as i64);
                res = self.i64_to_i44(r + 1, res + prod3 as i64);
            }

            self.mac[r + 1] = (res >> sf) as i32;
        }

        self.mac_to_ir(fields);
    }

    fn mac_to_ir(&mut self, fields: CommandFields) {
        let lm = fields.lm();
        for i in 1..=3 {
            self.ir[i] = self.i32_to_i16(i, self.mac[i], lm);
        }
    }

    fn mac_to_color_push(&mut self) {
        let color = [
            self.i32_to_u8(0, self.mac[1] >> 4),
            self.i32_to_u8(1, self.mac[2] >> 4),
            self.i32_to_u8(2, self.mac[3] >> 4),
            self.rgbc[3],
        ];
        self.colors.push(color);
    }
}
