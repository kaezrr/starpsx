use super::CommandFields;
use super::ControlVec;
use super::GTEngine;
use super::Matrix;
use super::Saturation;
use super::Vector;
use super::utils;

impl GTEngine {
    /// Perspective transformation(single)
    pub fn rtps(&mut self, fields: CommandFields) {
        let projection_factor = self.do_rtp(fields, Vector::V0);
        self.depth_queuing(projection_factor);
    }

    /// Normal clipping
    pub fn nclip(&mut self) {
        let [x0, y0] = self.sxy[0];
        let [x1, y1] = self.sxy[1];
        let [x2, y2] = self.sxy[2];

        let (x0, y0) = (i64::from(x0), i64::from(y0));
        let (x1, y1) = (i64::from(x1), i64::from(y1));
        let (x2, y2) = (i64::from(x2), i64::from(y2));

        let a = x0 * (y1 - y2);
        let b = x1 * (y2 - y0);
        let c = x2 * (y0 - y1);

        let sum = a + b + c;

        self.mac[0] = sum as i32;
        self.mac0_overflow_check(sum);
    }

    /// Cross product of two vectors
    pub fn op(&mut self, fields: CommandFields) {
        let [_, ir1, ir2, ir3] = self.ir;
        let (ir1, ir2, ir3) = (i32::from(ir1), i32::from(ir2), i32::from(ir3));

        let sf = fields.sf() * 12;

        let d1 = i32::from(self.rtm[0][0]);
        let d2 = i32::from(self.rtm[1][1]);
        let d3 = i32::from(self.rtm[2][2]);

        self.mac[1] = (d2 * ir3 - d3 * ir2) >> sf;
        self.mac[2] = (d3 * ir1 - d1 * ir3) >> sf;
        self.mac[3] = (d1 * ir2 - d2 * ir1) >> sf;

        self.mac_to_ir(fields);
    }

    /// Depth cueing (single)
    pub fn dpcs(&mut self, fields: CommandFields) {
        let dpc_vec = [
            i64::from(self.rgbc[0]),
            i64::from(self.rgbc[1]),
            i64::from(self.rgbc[2]),
        ];

        self.do_dpc(dpc_vec, 16, fields);
    }

    /// Interpolation of a vector and far color
    pub fn intpl(&mut self, fields: CommandFields) {
        let dpc_vec = [
            i64::from(self.ir[1]),
            i64::from(self.ir[2]),
            i64::from(self.ir[3]),
        ];

        self.do_dpc(dpc_vec, 12, fields);
    }

    /// Multiply vector by matrix and vector addition
    pub fn mvmva(&mut self, fields: CommandFields) {
        self.multiply_matrix_by_vector(fields, fields.mx(), fields.vx(), fields.cv());
    }

    /// Normal color depth cue single vector
    pub fn ncds(&mut self, fields: CommandFields) {
        self.multiply_matrix_by_vector(fields, Matrix::Light, Vector::V0, ControlVec::None);
        self.multiply_matrix_by_vector(fields, Matrix::Color, Vector::IR, ControlVec::Background);

        self.dcpl(fields);
    }

    /// Color depth cue
    pub fn cdp(&mut self, fields: CommandFields) {
        self.multiply_matrix_by_vector(fields, Matrix::Color, Vector::IR, ControlVec::Background);

        self.dcpl(fields);
    }

    /// Normal color depth cue triple vectors
    pub fn ncdt(&mut self, fields: CommandFields) {
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
        self.multiply_matrix_by_vector(fields, Matrix::Light, Vector::V0, ControlVec::None);
        self.cc(fields);
    }

    /// Color color
    pub fn cc(&mut self, fields: CommandFields) {
        self.multiply_matrix_by_vector(fields, Matrix::Color, Vector::IR, ControlVec::Background);

        let r = i64::from(self.rgbc[0]);
        let g = i64::from(self.rgbc[1]);
        let b = i64::from(self.rgbc[2]);

        let ir1 = i64::from(self.ir[1]);
        let ir2 = i64::from(self.ir[2]);
        let ir3 = i64::from(self.ir[3]);

        let shaded1 = (r * ir1) << 4;
        let shaded2 = (g * ir2) << 4;
        let shaded3 = (b * ir3) << 4;

        let sf = fields.sf() * 12;
        self.mac[1] = (self.i64_to_i44::<1>(shaded1) >> sf) as i32;
        self.mac[2] = (self.i64_to_i44::<2>(shaded2) >> sf) as i32;
        self.mac[3] = (self.i64_to_i44::<3>(shaded3) >> sf) as i32;

        self.mac_to_ir(fields);
        self.mac_to_color_push();
    }

    /// Normal color single
    pub fn ncs(&mut self, fields: CommandFields) {
        self.multiply_matrix_by_vector(fields, Matrix::Light, Vector::V0, ControlVec::None);
        self.multiply_matrix_by_vector(fields, Matrix::Color, Vector::IR, ControlVec::Background);

        self.mac_to_ir(fields);
        self.mac_to_color_push();
    }

    /// Normal color triple
    pub fn nct(&mut self, fields: CommandFields) {
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
        let sf = fields.sf() * 12;

        let prod1 = i32::from(self.ir[1]) * i32::from(self.ir[1]);
        let prod2 = i32::from(self.ir[2]) * i32::from(self.ir[2]);
        let prod3 = i32::from(self.ir[3]) * i32::from(self.ir[3]);

        self.mac[1] = prod1 >> sf;
        self.mac[2] = prod2 >> sf;
        self.mac[3] = prod3 >> sf;

        self.mac_to_ir(fields);
    }

    /// Depth cue color light
    pub fn dcpl(&mut self, fields: CommandFields) {
        let r = i64::from(self.rgbc[0]);
        let g = i64::from(self.rgbc[1]);
        let b = i64::from(self.rgbc[2]);

        let rfc = i64::from(self.fc[0]) << 12;
        let gfc = i64::from(self.fc[1]) << 12;
        let bfc = i64::from(self.fc[2]) << 12;

        let ir1 = i64::from(self.ir[1]);
        let ir2 = i64::from(self.ir[2]);
        let ir3 = i64::from(self.ir[3]);

        let shaded1 = (r * ir1) << 4;
        let shaded2 = (g * ir2) << 4;
        let shaded3 = (b * ir3) << 4;

        let sf = fields.sf() * 12;

        let sub1 = (self.i64_to_i44::<1>(rfc - shaded1) >> sf) as i32;
        let sub2 = (self.i64_to_i44::<2>(gfc - shaded2) >> sf) as i32;
        let sub3 = (self.i64_to_i44::<3>(bfc - shaded3) >> sf) as i32;

        let sat1 = i64::from(self.i32_to_i16::<1>(sub1, Saturation::S16));
        let sat2 = i64::from(self.i32_to_i16::<2>(sub2, Saturation::S16));
        let sat3 = i64::from(self.i32_to_i16::<3>(sub3, Saturation::S16));

        let ir0 = i64::from(self.ir[0]);

        self.mac[1] = (self.i64_to_i44::<1>(shaded1 + ir0 * sat1) >> sf) as i32;
        self.mac[2] = (self.i64_to_i44::<2>(shaded2 + ir0 * sat2) >> sf) as i32;
        self.mac[3] = (self.i64_to_i44::<3>(shaded3 + ir0 * sat3) >> sf) as i32;

        self.mac_to_ir(fields);
        self.mac_to_color_push();
    }

    /// Depth cueing (triple)
    pub fn dpct(&mut self, fields: CommandFields) {
        for _ in 0..3 {
            let dpc_vec = [
                i64::from(self.colors[0][0]),
                i64::from(self.colors[0][1]),
                i64::from(self.colors[0][2]),
            ];

            self.do_dpc(dpc_vec, 16, fields);
        }
    }

    /// Average of three Z values (for triangles)
    pub fn avsz3(&mut self) {
        let z1 = u32::from(self.sz.fifo[1]);
        let z2 = u32::from(self.sz.fifo[2]);
        let z3 = u32::from(self.sz.fifo[3]);

        let sum = z1 + z2 + z3;
        let average = i64::from(self.zsf3) * i64::from(sum);

        self.mac[0] = average as i32;
        self.mac0_overflow_check(average);

        self.otz = self.i64_to_otz(average);
    }

    /// Average of four Z values (for quads)
    pub fn avsz4(&mut self) {
        let z0 = u32::from(self.sz.fifo[0]);
        let z1 = u32::from(self.sz.fifo[1]);
        let z2 = u32::from(self.sz.fifo[2]);
        let z3 = u32::from(self.sz.fifo[3]);

        let sum = z0 + z1 + z2 + z3;
        let average = i64::from(self.zsf4) * i64::from(sum);

        self.mac[0] = average as i32;
        self.mac0_overflow_check(average);

        self.otz = self.i64_to_otz(average);
    }

    /// Perspective transformation (triple)
    pub fn rtpt(&mut self, fields: CommandFields) {
        self.do_rtp(fields, Vector::V0);
        self.do_rtp(fields, Vector::V1);

        // We do depth queuing on the last vector
        let projection_factor = self.do_rtp(fields, Vector::V2);

        self.depth_queuing(projection_factor);
    }

    /// General purpose interpolation
    pub fn gpf(&mut self, fields: CommandFields) {
        let sf = fields.sf() * 12;
        let ir0 = i32::from(self.ir[0]);

        let prod1 = i32::from(self.ir[1]) * ir0;
        let prod2 = i32::from(self.ir[2]) * ir0;
        let prod3 = i32::from(self.ir[3]) * ir0;

        self.mac[1] = (self.i64_to_i44::<1>(i64::from(prod1)) >> sf) as i32;
        self.mac[2] = (self.i64_to_i44::<2>(i64::from(prod2)) >> sf) as i32;
        self.mac[3] = (self.i64_to_i44::<3>(i64::from(prod3)) >> sf) as i32;

        self.mac_to_ir(fields);
        self.mac_to_color_push();
    }

    /// General purpose interpolation with base
    pub fn gpl(&mut self, fields: CommandFields) {
        let sf = fields.sf() * 12;
        let ir0 = i32::from(self.ir[0]);

        let mac1 = i64::from(self.mac[1]) << sf;
        let mac2 = i64::from(self.mac[2]) << sf;
        let mac3 = i64::from(self.mac[3]) << sf;

        let prod1 = i32::from(self.ir[1]) * ir0;
        let prod2 = i32::from(self.ir[2]) * ir0;
        let prod3 = i32::from(self.ir[3]) * ir0;

        self.mac[1] = (self.i64_to_i44::<1>(mac1 + i64::from(prod1)) >> sf) as i32;
        self.mac[2] = (self.i64_to_i44::<2>(mac2 + i64::from(prod2)) >> sf) as i32;
        self.mac[3] = (self.i64_to_i44::<3>(mac3 + i64::from(prod3)) >> sf) as i32;

        self.mac_to_ir(fields);
        self.mac_to_color_push();
    }

    /// Normal color color (triple vector)
    pub fn ncct(&mut self, fields: CommandFields) {
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
        self.ir[1] = self.i32_to_i16::<1>(self.mac[1], lm);
        self.ir[2] = self.i32_to_i16::<2>(self.mac[2], lm);
        self.ir[3] = self.i32_to_i16::<3>(self.mac[3], lm);
    }

    fn mac_to_color_push(&mut self) {
        let color = [
            self.i32_to_u8::<0>(self.mac[1] >> 4),
            self.i32_to_u8::<1>(self.mac[2] >> 4),
            self.i32_to_u8::<2>(self.mac[3] >> 4),
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

    fn multiply_matrix_by_vector(
        &mut self,
        fields: CommandFields,
        mx: Matrix,
        vx: Vector,
        cv: ControlVec,
    ) {
        let mx = match mx {
            Matrix::Rotation => self.rtm,
            Matrix::Light => self.llm,
            Matrix::Color => self.lcm,
            Matrix::Reserved => {
                let r13 = self.rtm[0][2];
                let r22 = self.rtm[1][1];
                let red = i16::from(self.rgbc[0]);
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
            ControlVec::Translation => self.tr,
            ControlVec::Background => self.bk,
            ControlVec::FarColor => self.fc,
            ControlVec::None => [0, 0, 0],
        };

        let sf = fields.sf() * 12;
        let mut temp = [0; 3];

        let tr_x = i64::from(tr[0]) << 12;
        let tr_y = i64::from(tr[1]) << 12;
        let tr_z = i64::from(tr[2]) << 12;

        temp[0] = self.i64_to_i44::<1>(tr_x + i64::from(mx[0][0]) * i64::from(v[0]));
        temp[1] = self.i64_to_i44::<2>(tr_y + i64::from(mx[1][0]) * i64::from(v[0]));
        temp[2] = self.i64_to_i44::<3>(tr_z + i64::from(mx[2][0]) * i64::from(v[0]));

        if matches!(cv, ControlVec::FarColor) {
            self.i32_to_i16::<1>((temp[0] >> sf) as i32, Saturation::S16);
            self.i32_to_i16::<2>((temp[1] >> sf) as i32, Saturation::S16);
            self.i32_to_i16::<3>((temp[2] >> sf) as i32, Saturation::S16);

            temp[0] = 0;
            temp[1] = 0;
            temp[2] = 0;
        }

        temp[0] = self.i64_to_i44::<1>(temp[0] + i64::from(mx[0][1]) * i64::from(v[1]));
        temp[1] = self.i64_to_i44::<2>(temp[1] + i64::from(mx[1][1]) * i64::from(v[1]));
        temp[2] = self.i64_to_i44::<3>(temp[2] + i64::from(mx[2][1]) * i64::from(v[1]));

        temp[0] = self.i64_to_i44::<1>(temp[0] + i64::from(mx[0][2]) * i64::from(v[2]));
        temp[1] = self.i64_to_i44::<2>(temp[1] + i64::from(mx[1][2]) * i64::from(v[2]));
        temp[2] = self.i64_to_i44::<3>(temp[2] + i64::from(mx[2][2]) * i64::from(v[2]));

        self.mac[1] = (temp[0] >> sf) as i32;
        self.mac[2] = (temp[1] >> sf) as i32;
        self.mac[3] = (temp[2] >> sf) as i32;

        self.mac_to_ir(fields);
    }

    fn do_rtp(&mut self, fields: CommandFields, vec: Vector) -> u32 {
        let sf = fields.sf() * 12;
        let mut temp = [0; 3];

        let tr_x = i64::from(self.tr[0]) << 12;
        let tr_y = i64::from(self.tr[1]) << 12;
        let tr_z = i64::from(self.tr[2]) << 12;

        let vec = self.v[vec as usize];
        let mx = self.rtm;

        temp[0] = self.i64_to_i44::<1>(tr_x + i64::from(mx[0][0]) * i64::from(vec[0]));
        temp[1] = self.i64_to_i44::<2>(tr_y + i64::from(mx[1][0]) * i64::from(vec[0]));
        temp[2] = self.i64_to_i44::<3>(tr_z + i64::from(mx[2][0]) * i64::from(vec[0]));

        temp[0] = self.i64_to_i44::<1>(temp[0] + i64::from(mx[0][1]) * i64::from(vec[1]));
        temp[1] = self.i64_to_i44::<2>(temp[1] + i64::from(mx[1][1]) * i64::from(vec[1]));
        temp[2] = self.i64_to_i44::<3>(temp[2] + i64::from(mx[2][1]) * i64::from(vec[1]));

        temp[0] = self.i64_to_i44::<1>(temp[0] + i64::from(mx[0][2]) * i64::from(vec[2]));
        temp[1] = self.i64_to_i44::<2>(temp[1] + i64::from(mx[1][2]) * i64::from(vec[2]));
        temp[2] = self.i64_to_i44::<3>(temp[2] + i64::from(mx[2][2]) * i64::from(vec[2]));

        self.mac[1] = (temp[0] >> sf) as i32;
        self.mac[2] = (temp[1] >> sf) as i32;
        self.mac[3] = (temp[2] >> sf) as i32;

        // Dont update IR3 here
        let lm = fields.lm();
        self.ir[1] = self.i32_to_i16::<1>(self.mac[1], lm);
        self.ir[2] = self.i32_to_i16::<2>(self.mac[2], lm);

        // Special IR3 bug handling
        let min = i16::MIN.into();
        let max = i16::MAX.into();

        let last_z = (temp[2] >> 12) as i32;

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
        } else if last_z > i32::from(u16::MAX) {
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

        let factor = i64::from(projection_factor);
        let (x, y) = (i64::from(self.ir[1]), i64::from(self.ir[2]));
        let (ofx, ofy) = (i64::from(self.of[0]), i64::from(self.of[1]));

        let screen_x = x * factor + ofx;
        let screen_y = y * factor + ofy;

        self.mac0_overflow_check(screen_x);
        self.mac0_overflow_check(screen_y);

        let sx2 = self.i32_to_i11::<0>((screen_x >> 16) as i32);
        let sy2 = self.i32_to_i11::<1>((screen_y >> 16) as i32);

        self.sxy.push([sx2, sy2]);

        projection_factor
    }

    fn depth_queuing(&mut self, projection_factor: u32) {
        let (dqa, dqb) = (i64::from(self.dqa), i64::from(self.dqb));
        let factor = i64::from(projection_factor);

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

    fn do_dpc(&mut self, vec: [i64; 3], shift: u8, fields: CommandFields) {
        let sf = fields.sf() * 12;
        let ir0 = i64::from(self.ir[0]);

        let x = vec[0] << shift;
        let y = vec[1] << shift;
        let z = vec[2] << shift;

        let rfc = i64::from(self.fc[0]) << 12;
        let gfc = i64::from(self.fc[1]) << 12;
        let bfc = i64::from(self.fc[2]) << 12;

        let sub1 = (self.i64_to_i44::<1>(rfc - x) >> sf) as i32;
        let sub3 = (self.i64_to_i44::<3>(bfc - z) >> sf) as i32;
        let sub2 = (self.i64_to_i44::<2>(gfc - y) >> sf) as i32;

        let sat1 = i64::from(self.i32_to_i16::<1>(sub1, Saturation::S16));
        let sat2 = i64::from(self.i32_to_i16::<2>(sub2, Saturation::S16));
        let sat3 = i64::from(self.i32_to_i16::<3>(sub3, Saturation::S16));

        self.mac[1] = (self.i64_to_i44::<1>(x + ir0 * sat1) >> sf) as i32;
        self.mac[2] = (self.i64_to_i44::<2>(y + ir0 * sat2) >> sf) as i32;
        self.mac[3] = (self.i64_to_i44::<3>(z + ir0 * sat3) >> sf) as i32;

        self.mac_to_ir(fields);
        self.mac_to_color_push();
    }

    fn i64_to_i44<const INDEX: usize>(&mut self, val: i64) -> i64 {
        if val > 0x7FF_FFFF_FFFF {
            match INDEX {
                1 => self.flag.mac1_overflow_pos(true),
                2 => self.flag.mac2_overflow_pos(true),
                3 => self.flag.mac3_overflow_pos(true),
                _ => unreachable!(),
            }
        } else if val < -0x800_0000_0000 {
            match INDEX {
                1 => self.flag.mac1_overflow_neg(true),
                2 => self.flag.mac2_overflow_neg(true),
                3 => self.flag.mac3_overflow_neg(true),
                _ => unreachable!(),
            }
        }

        (val << 20) >> 20
    }

    fn i32_to_i16<const INDEX: usize>(&mut self, val: i32, lm: Saturation) -> i16 {
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
            match INDEX {
                1 => self.flag.ir1_saturated(true),
                2 => self.flag.ir2_saturated(true),
                3 => self.flag.ir3_saturated(true),
                _ => unreachable!(),
            }
        }

        res
    }

    fn i32_to_u8<const INDEX: usize>(&mut self, val: i32) -> u8 {
        let (min, max) = (u8::MIN.into(), u8::MAX.into());

        let (res, saturated) = if val > max {
            (max as u8, true)
        } else if val < min {
            (min as u8, true)
        } else {
            (val as u8, false)
        };

        if saturated {
            match INDEX {
                0 => self.flag.cfifo_r_saturated(true),
                1 => self.flag.cfifo_g_saturated(true),
                2 => self.flag.cfifo_b_saturated(true),
                _ => unreachable!(),
            }
        }

        res
    }

    fn i32_to_i11<const INDEX: usize>(&mut self, val: i32) -> i16 {
        let (res, saturated) = if val < -0x400 {
            (-0x400, true)
        } else if val > 0x3FF {
            (0x3FF, true)
        } else {
            (val, false)
        };

        if saturated {
            match INDEX {
                0 => self.flag.sx2_saturated(true),
                1 => self.flag.sy2_saturated(true),
                _ => unreachable!(),
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
}
