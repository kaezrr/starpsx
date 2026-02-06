use tracing::debug;

use super::*;

impl GTEngine {
    /// Perspective transformation(single)
    pub fn rtps(&mut self, fields: CommandFields) {
        debug!(target:"gte","gte command, rtps");

        let projection_factor = self.do_rtp(fields, Vector::V0);
        self.depth_queuing(projection_factor);
    }

    /// Normal clipping
    pub fn nclip(&mut self) {
        debug!(target:"gte","gte command, nclip");

        let [x0, y0] = self.sxy[0];
        let [x1, y1] = self.sxy[1];
        let [x2, y2] = self.sxy[2];

        let (x0, y0) = (x0 as i64, y0 as i64);
        let (x1, y1) = (x1 as i64, y1 as i64);
        let (x2, y2) = (x2 as i64, y2 as i64);

        let a = x0 * (y1 - y2);
        let b = x1 * (y2 - y0);
        let c = x2 * (y0 - y1);

        let sum = a + b + c;

        self.mac[0] = sum as i32;
        self.mac0_overflow_check(sum);
    }

    /// Cross product of two vectors
    pub fn op(&mut self, fields: CommandFields) {
        debug!(target:"gte","gte command, op");

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
        debug!(target:"gte","gte command, dpcs");

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
        debug!(target:"gte","gte command, intpl");

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
        debug!(target:"gte","gte command, mvmva");
        self.multiply_matrix_by_vector(fields, fields.mx(), fields.vx(), fields.cv());
    }

    /// Normal color depth cue single vector
    pub fn ncds(&mut self, fields: CommandFields) {
        debug!(target:"gte","gte command, ncds");

        self.multiply_matrix_by_vector(fields, Matrix::Light, Vector::V0, ControlVec::None);
        self.multiply_matrix_by_vector(fields, Matrix::Color, Vector::IR, ControlVec::Background);

        self.dcpl(fields);
    }

    /// Color depth cue
    pub fn cdp(&mut self, fields: CommandFields) {
        debug!(target:"gte","gte command, cdp");

        self.multiply_matrix_by_vector(fields, Matrix::Color, Vector::IR, ControlVec::Background);

        self.dcpl(fields);
    }

    /// Normal color depth cue triple vectors
    pub fn ncdt(&mut self, fields: CommandFields) {
        debug!(target:"gte","gte command, ncdt");

        self.multiply_matrix_by_vector(fields, Matrix::Light, Vector::V0, ControlVec::None);
        self.multiply_matrix_by_vector(fields, Matrix::Color, Vector::IR, ControlVec::Background);

        self.dcpl(fields);

        self.multiply_matrix_by_vector(fields, Matrix::Light, Vector::V1, ControlVec::None);
        self.multiply_matrix_by_vector(fields, Matrix::Color, Vector::IR, ControlVec::Background);

        self.dcpl(fields);

        self.multiply_matrix_by_vector(fields, Matrix::Light, Vector::V2, ControlVec::None);
        self.multiply_matrix_by_vector(fields, Matrix::Color, Vector::IR, ControlVec::Background);

        self.dcpl(fields);
    }

    /// Normal color color single vector
    pub fn nccs(&mut self, fields: CommandFields) {
        debug!(target:"gte","gte command, nccs");

        self.multiply_matrix_by_vector(fields, Matrix::Light, Vector::V0, ControlVec::None);
        self.cc(fields);
    }

    /// Color color
    pub fn cc(&mut self, fields: CommandFields) {
        debug!(target:"gte","gte command, cc");

        self.multiply_matrix_by_vector(fields, Matrix::Color, Vector::IR, ControlVec::Background);

        let sf = fields.sf() * 12;
        for i in 0..3 {
            let col = self.rgbc[i] as i64;
            let ir = self.ir[i + 1] as i64;

            let shaded = (col * ir) << 4;
            let res = self.i64_to_i44(i + 1, shaded);

            self.mac[i + 1] = (res >> sf) as i32;
        }

        self.mac_to_ir(fields);
        self.mac_to_color_push();
    }

    /// Normal color single
    pub fn ncs(&mut self, fields: CommandFields) {
        debug!(target:"gte","gte command, ncs");

        self.multiply_matrix_by_vector(fields, Matrix::Light, Vector::V0, ControlVec::None);
        self.multiply_matrix_by_vector(fields, Matrix::Color, Vector::IR, ControlVec::Background);

        self.mac_to_ir(fields);
        self.mac_to_color_push();
    }

    /// Normal color triple
    pub fn nct(&mut self, fields: CommandFields) {
        debug!(target:"gte","gte command, nct");

        self.multiply_matrix_by_vector(fields, Matrix::Light, Vector::V0, ControlVec::None);
        self.multiply_matrix_by_vector(fields, Matrix::Color, Vector::IR, ControlVec::Background);

        self.mac_to_ir(fields);
        self.mac_to_color_push();

        self.multiply_matrix_by_vector(fields, Matrix::Light, Vector::V1, ControlVec::None);
        self.multiply_matrix_by_vector(fields, Matrix::Color, Vector::IR, ControlVec::Background);

        self.mac_to_ir(fields);
        self.mac_to_color_push();

        self.multiply_matrix_by_vector(fields, Matrix::Light, Vector::V2, ControlVec::None);
        self.multiply_matrix_by_vector(fields, Matrix::Color, Vector::IR, ControlVec::Background);

        self.mac_to_ir(fields);
        self.mac_to_color_push();
    }

    /// Square vector
    pub fn sqr(&mut self, fields: CommandFields) {
        debug!(target:"gte","gte command, sqr");

        let sf = fields.sf() * 12;

        let prod1 = self.ir[1] as i32 * self.ir[1] as i32;
        let prod2 = self.ir[2] as i32 * self.ir[2] as i32;
        let prod3 = self.ir[3] as i32 * self.ir[3] as i32;

        self.mac[1] = prod1 >> sf;
        self.mac[2] = prod2 >> sf;
        self.mac[3] = prod3 >> sf;

        self.mac_to_ir(fields);
    }

    /// Depth cue color light
    pub fn dcpl(&mut self, fields: CommandFields) {
        debug!(target:"gte","gte command, dcpl");

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
    pub fn dpct(&mut self, fields: CommandFields) {
        debug!(target:"gte","gte command, dpct");

        for _ in 0..3 {
            let sf = fields.sf() * 12;
            let far_colors = self.control_vectors[ControlVec::FarColor as usize];

            far_colors.iter().enumerate().for_each(|(i, &fc)| {
                let c = (self.colors[0][i] as i64) << 16;
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
    }

    /// Average of three Z values (for triangles)
    pub fn avsz3(&mut self) {
        debug!(target:"gte","gte command, avsz3");

        let z1 = self.sz.fifo[1] as u32;
        let z2 = self.sz.fifo[2] as u32;
        let z3 = self.sz.fifo[3] as u32;

        let sum = z1 + z2 + z3;
        let average = self.zsf3 as i64 * sum as i64;

        self.mac[0] = average as i32;
        self.mac0_overflow_check(average);

        self.otz = self.i64_to_otz(average);
    }

    /// Average of four Z values (for quads)
    pub fn avsz4(&mut self) {
        debug!(target:"gte","gte command, avsz4");

        let z0 = self.sz.fifo[0] as u32;
        let z1 = self.sz.fifo[1] as u32;
        let z2 = self.sz.fifo[2] as u32;
        let z3 = self.sz.fifo[3] as u32;

        let sum = z0 + z1 + z2 + z3;
        let average = self.zsf4 as i64 * sum as i64;

        self.mac[0] = average as i32;
        self.mac0_overflow_check(average);

        self.otz = self.i64_to_otz(average);
    }

    /// Perspective transformation (triple)
    pub fn rtpt(&mut self, fields: CommandFields) {
        debug!(target:"gte","gte command, rtpt");

        self.do_rtp(fields, Vector::V0);
        self.do_rtp(fields, Vector::V1);

        // We do depth queuing on the last vector
        let projection_factor = self.do_rtp(fields, Vector::V2);

        self.depth_queuing(projection_factor);
    }

    /// General purpose interpolation
    pub fn gpf(&mut self, fields: CommandFields) {
        debug!(target:"gte","gte command, gpf");

        let sf = fields.sf() * 12;
        let ir0 = self.ir[0] as i32;

        for i in 0..3 {
            let prod = self.ir[i + 1] as i32 * ir0;
            let res = self.i64_to_i44(i + 1, prod as i64);

            self.mac[i + 1] = (res >> sf) as i32;
        }

        self.mac_to_ir(fields);
        self.mac_to_color_push();
    }

    /// General purpose interpolation with base
    pub fn gpl(&mut self, fields: CommandFields) {
        debug!(target:"gte","gte command, gpl");

        let sf = fields.sf() * 12;
        let ir0 = self.ir[0] as i32;

        for i in 0..3 {
            let mac = (self.mac[i + 1] as i64) << sf;
            let prod = self.ir[i + 1] as i32 * ir0;
            let res = self.i64_to_i44(i + 1, mac + prod as i64);

            self.mac[i + 1] = (res >> sf) as i32;
        }

        self.mac_to_ir(fields);
        self.mac_to_color_push();
    }

    /// Normal color color (triple vector)
    pub fn ncct(&mut self, fields: CommandFields) {
        debug!(target:"gte","gte command, ncct");

        self.multiply_matrix_by_vector(fields, Matrix::Light, Vector::V0, ControlVec::None);
        self.cc(fields);

        self.multiply_matrix_by_vector(fields, Matrix::Light, Vector::V1, ControlVec::None);
        self.cc(fields);

        self.multiply_matrix_by_vector(fields, Matrix::Light, Vector::V2, ControlVec::None);
        self.cc(fields);
    }

    // --------------------------Helper Flag and Math Functions------------------------- //

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

    fn mac0_overflow_check(&mut self, val: i64) {
        if val > i32::MAX.into() {
            self.flag.mac0_overflow_pos(true);
        } else if val < i32::MIN.into() {
            self.flag.mac0_overflow_neg(true);
        }
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

    fn i32_to_i11(&mut self, is_x: bool, val: i32) -> i16 {
        let (res, saturated) = if val < -0x400 {
            (-0x400, true)
        } else if val > 0x3FF {
            (0x3FF, true)
        } else {
            (val, false)
        };

        if saturated {
            match is_x {
                true => self.flag.sx2_saturated(true),
                false => self.flag.sy2_saturated(true),
            }
        }

        res as i16
    }

    fn i64_to_otz(&mut self, val: i64) -> u16 {
        let val = val >> 12;

        let (res, saturated) = if val < 0 {
            (0, true)
        } else if val > 0xFFFF {
            (0xFFFF, true)
        } else {
            (val, false)
        };

        if saturated {
            self.flag.sz3_or_otz_saturated(true);
        }

        res as u16
    }

    fn multiply_matrix_by_vector(
        &mut self,
        fields: CommandFields,
        mx: Matrix,
        vx: Vector,
        cv: ControlVec,
    ) {
        let mx = match mx {
            Matrix::Rotation => self.matrices[0],
            Matrix::Light => self.matrices[1],
            Matrix::Color => self.matrices[2],
            Matrix::Reserved => {
                let r13 = self.matrices[0][0][2];
                let r22 = self.matrices[0][1][1];
                let red = self.rgbc[0] as i16;
                let ir0 = self.ir[0];

                // Garbage matrix
                [
                    [(-red) << 4, red << 4, ir0],
                    [r13, r13, r13],
                    [r22, r22, r22],
                ]
            }
        };

        let v = match vx {
            Vector::V0 => self.v[0],
            Vector::V1 => self.v[1],
            Vector::V2 => self.v[2],
            Vector::IR => [self.ir[1], self.ir[2], self.ir[3]],
        };

        let tr = match cv {
            ControlVec::Translation => self.control_vectors[0],
            ControlVec::Background => self.control_vectors[1],
            ControlVec::FarColor => self.control_vectors[2],
            ControlVec::None => [0, 0, 0],
        };

        let sf = fields.sf() * 12;
        let mut temp = [0; 3];

        let tr_x = (tr[0] as i64) << 12;
        let tr_y = (tr[1] as i64) << 12;
        let tr_z = (tr[2] as i64) << 12;

        temp[0] = self.i64_to_i44(1, tr_x + mx[0][0] as i64 * v[0] as i64);
        temp[1] = self.i64_to_i44(2, tr_y + mx[1][0] as i64 * v[0] as i64);
        temp[2] = self.i64_to_i44(3, tr_z + mx[2][0] as i64 * v[0] as i64);

        if matches!(cv, ControlVec::FarColor) {
            self.i32_to_i16(1, (temp[0] >> sf) as i32, Saturation::S16);
            self.i32_to_i16(2, (temp[1] >> sf) as i32, Saturation::S16);
            self.i32_to_i16(3, (temp[2] >> sf) as i32, Saturation::S16);

            temp[0] = 0;
            temp[1] = 0;
            temp[2] = 0;
        }

        temp[0] = self.i64_to_i44(1, temp[0] + mx[0][1] as i64 * v[1] as i64);
        temp[1] = self.i64_to_i44(2, temp[1] + mx[1][1] as i64 * v[1] as i64);
        temp[2] = self.i64_to_i44(3, temp[2] + mx[2][1] as i64 * v[1] as i64);

        temp[0] = self.i64_to_i44(1, temp[0] + mx[0][2] as i64 * v[2] as i64);
        temp[1] = self.i64_to_i44(2, temp[1] + mx[1][2] as i64 * v[2] as i64);
        temp[2] = self.i64_to_i44(3, temp[2] + mx[2][2] as i64 * v[2] as i64);

        self.mac[1] = (temp[0] >> sf) as i32;
        self.mac[2] = (temp[1] >> sf) as i32;
        self.mac[3] = (temp[2] >> sf) as i32;

        self.mac_to_ir(fields);
    }

    fn do_rtp(&mut self, fields: CommandFields, vec: Vector) -> u32 {
        let sf = fields.sf();
        let vx = vec as usize;

        let mut last_z = 0;
        for r in 0..3 {
            let prod1 = (self.v[vx][0] as i32) * (self.matrices[0][r][0] as i32);
            let prod2 = (self.v[vx][1] as i32) * (self.matrices[0][r][1] as i32);
            let prod3 = (self.v[vx][2] as i32) * (self.matrices[0][r][2] as i32);

            let mut res = (self.control_vectors[0][r] as i64) << 12;
            res = self.i64_to_i44(r + 1, res + prod1 as i64);
            res = self.i64_to_i44(r + 1, res + prod2 as i64);
            res = self.i64_to_i44(r + 1, res + prod3 as i64);

            self.mac[r + 1] = (res >> (12 * sf)) as i32;
            last_z = (res >> 12) as i32;
        }

        // Dont update IR3 here
        let lm = fields.lm();
        for i in 1..=2 {
            self.ir[i] = self.i32_to_i16(i, self.mac[i], lm);
        }

        // Special IR3 bug handling
        let min = i16::MIN.into();
        let max = i16::MAX.into();

        if last_z > max || last_z < min {
            self.flag.ir3_saturated(true);
        }

        let min = match lm {
            Saturation::S16 => i16::MIN.into(),
            Saturation::U15 => 0x0000,
        };

        let val = self.mac[3];
        self.ir[3] = if val < min {
            min as i16
        } else if val > max {
            max as i16
        } else {
            val as i16
        };

        let sz3 = if last_z < 0 {
            self.flag.sz3_or_otz_saturated(true);
            0
        } else if last_z > u16::MAX as i32 {
            self.flag.sz3_or_otz_saturated(true);
            u16::MAX
        } else {
            last_z as u16
        };

        self.sz.push(sz3);

        let projection_factor = if sz3 > self.h / 2 {
            utils::divide(self.h, sz3)
        } else {
            self.flag.div_overflow(true);
            0x1FFFF
        };

        let factor = projection_factor as i64;
        let (x, y) = (self.ir[1] as i64, self.ir[2] as i64);
        let (ofx, ofy) = (self.of[0] as i64, self.of[1] as i64);

        let screen_x = x * factor + ofx;
        let screen_y = y * factor + ofy;

        self.mac0_overflow_check(screen_x);
        self.mac0_overflow_check(screen_y);

        let sx2 = self.i32_to_i11(true, (screen_x >> 16) as i32);
        let sy2 = self.i32_to_i11(false, (screen_y >> 16) as i32);

        self.sxy.push([sx2, sy2]);

        projection_factor
    }

    fn depth_queuing(&mut self, projection_factor: u32) {
        let (dqa, dqb) = (self.dqa as i64, self.dqb as i64);
        let factor = projection_factor as i64;

        let res = dqb + dqa * factor;
        self.mac0_overflow_check(res);
        self.mac[0] = res as i32;

        let res = res >> 12;

        self.ir[0] = if res < 0 {
            self.flag.ir0_saturated(true);
            0x0000
        } else if res > 0x1000 {
            self.flag.ir0_saturated(true);
            0x1000
        } else {
            res as i16
        };
    }
}
