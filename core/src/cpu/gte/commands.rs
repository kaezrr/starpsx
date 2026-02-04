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

        let (x0, y0) = (x0 as i32, y0 as i32);
        let (x1, y1) = (x1 as i32, y1 as i32);
        let (x2, y2) = (x2 as i32, y2 as i32);

        let a = x0 * (y1 - y2);
        let b = x1 * (y2 - y0);
        let c = x2 * (y0 - y1);

        self.mac[0] = self.mac_32_clamp(a as i64 + b as i64 + c as i64);
    }

    /// Cross product of two vectors
    pub fn op(&mut self, fields: CommandFields) {
        debug!("gte command, op");

        let [_, ir1, ir2, ir3] = self.ir;

        let (ir1, ir2, ir3) = (ir1 as i64, ir2 as i64, ir3 as i64);

        let lm = fields.lm();
        let sf = fields.sf() * 12;

        let d1 = self.rtm[0][0] as i64;
        let d2 = self.rtm[1][1] as i64;
        let d3 = self.rtm[2][2] as i64;

        self.mac[1] = self.mac_44_clamp(1, d2 * ir3 - d3 * ir2) >> sf;
        self.mac[2] = self.mac_44_clamp(2, d3 * ir1 - d1 * ir3) >> sf;
        self.mac[3] = self.mac_44_clamp(3, d1 * ir2 - d2 * ir1) >> sf;

        self.ir[1] = self.mac_to_ir(lm, 1);
        self.ir[2] = self.mac_to_ir(lm, 2);
        self.ir[3] = self.mac_to_ir(lm, 3);
    }

    /// Depth cueing (single)
    pub fn dpcs(&mut self, cmd: CommandFields) {
        debug!("gte command, dpcs");
    }

    ///
    pub fn intpl(&mut self) {
        debug!("gte command, intpl");
    }

    ///
    pub fn mvmva(&mut self) {
        debug!("gte command, mvmva");
    }

    /// Normal color depth cue single vector
    pub fn ncds(&mut self) {
        debug!("gte command, ncds");
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

    ///
    pub fn dcpl(&mut self) {
        debug!("gte command, dcpl");
    }

    ///
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

    fn mac_32_clamp(&mut self, mac: i64) -> i64 {
        if mac > i32::MAX.into() {
            self.flag.mac0_overflow_pos(true);
        } else if mac < i32::MIN.into() {
            self.flag.mac0_overflow_neg(true);
        }

        (mac << 32) >> 32
    }

    fn mac_44_clamp(&mut self, mac_index: usize, mac: i64) -> i64 {
        debug_assert!(matches!(mac_index, 1..=3));

        if mac > 0x7FFFFFFFFFF {
            match mac_index {
                1 => self.flag.mac1_overflow_pos(true),
                2 => self.flag.mac2_overflow_pos(true),
                3 => self.flag.mac3_overflow_pos(true),
                _ => unreachable!(),
            }
        } else if mac < -0x80000000000 {
            match mac_index {
                1 => self.flag.mac1_overflow_neg(true),
                2 => self.flag.mac2_overflow_neg(true),
                3 => self.flag.mac3_overflow_neg(true),
                _ => unreachable!(),
            }
        }

        (mac << 20) >> 20
    }

    fn mac_to_ir(&mut self, lm: SaturationRange, mac_index: usize) -> i16 {
        debug_assert!(matches!(mac_index, 1..=3));

        let min = match lm {
            SaturationRange::Unsigned15 => 0,
            SaturationRange::Signed16 => -0x8000,
        };

        let res = self.mac[mac_index];
        let (res, saturated) = if res > 0x7FFF {
            (0x7FFF, true)
        } else if res < min {
            (min, true)
        } else {
            (res, false)
        };

        match mac_index {
            1 => self.flag.ir1_saturated(saturated),
            2 => self.flag.ir2_saturated(saturated),
            3 => self.flag.ir3_saturated(saturated),
            _ => unreachable!(),
        }

        res as i16
    }
}
