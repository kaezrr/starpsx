use super::*;
use tracing::debug;

/// Check if mac0 postive or negative overflowed in 31 bits
fn check_flag_mac0(f: &mut Flag, v: i64) {
    if v.abs() > (1 << 31) {
        if v.is_positive() {
            f.mac0_overflow_pos(true);
        } else {
            f.mac0_overflow_neg(true);
        }
    }
}

/// Check if mac1/2/3 postive or negative overflowed in 43 bits
fn check_flag_macv(f: &mut Flag, v: i64, index: usize) {
    debug_assert!(matches!(index, 1..=3));

    if v.abs() > (1 << 43) {
        if v.is_positive() {
            match index {
                1 => f.mac1_overflow_pos(true),
                2 => f.mac2_overflow_pos(true),
                3 => f.mac3_overflow_pos(true),
                _ => unreachable!(),
            }
        } else {
            match index {
                1 => f.mac1_overflow_neg(true),
                2 => f.mac2_overflow_neg(true),
                3 => f.mac3_overflow_neg(true),
                _ => unreachable!(),
            }
        }
    }
}

impl GTEngine {
    /// Perspective transformation(single)
    pub fn rtps(&mut self, cmd: GteCommand) {
        debug!("gte command, rtps");
    }

    /// Normal clipping
    pub fn nclip(&mut self) {
        debug!("gte command, nclip");

        let s0 = self.sxy.fifo[0];
        let s1 = self.sxy.fifo[1];
        let s2 = self.sxy.fifo[2];

        self.mac0 = (s1 - s0).cross(s2 - s0);

        check_flag_mac0(&mut self.flag, self.mac0);
    }

    /// Cross product of two vectors
    pub fn op(&mut self, cmd: GteCommand) {
        debug!("gte command, op");

        let sf = cmd.sf();
        let lm = cmd.lm();

        let d = Vector3 {
            x: self.rtm.elems[0] as i64,
            y: self.rtm.elems[4] as i64,
            z: self.rtm.elems[8] as i64,
        };

        let ir = Vector3 {
            x: self.ir.x as i64,
            y: self.ir.y as i64,
            z: self.ir.z as i64,
        };

        self.macv = d.cross(ir) >> (sf * 12);

        let (v, ir_flags) = self.macv.saturated(lm);
        self.ir = v;

        self.flag.ir1_saturated(ir_flags[0]);
        self.flag.ir2_saturated(ir_flags[1]);
        self.flag.ir3_saturated(ir_flags[2]);

        check_flag_macv(&mut self.flag, self.macv.x, 1);
        check_flag_macv(&mut self.flag, self.macv.y, 2);
        check_flag_macv(&mut self.flag, self.macv.z, 3);
    }

    ///
    pub fn dpcs(&mut self) {
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
}
