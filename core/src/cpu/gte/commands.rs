use tracing::debug;

use super::*;
use util::check_flag_mac0;
use util::check_flag_macv;

impl GTEngine {
    /// Perspective transformation(single)
    pub fn rtps(&mut self) {
        debug!("gte command, rtps");
    }

    /// Normal clipping
    pub fn nclip(&mut self) {
        debug!("gte command, nclip");

        let s0 = Vector2 {
            x: self.sxy[0].x as i64,
            y: self.sxy[0].y as i64,
        };

        let s1 = Vector2 {
            x: self.sxy[1].x as i64,
            y: self.sxy[1].y as i64,
        };

        let s2 = Vector2 {
            x: self.sxy[2].x as i64,
            y: self.sxy[2].y as i64,
        };

        self.mac0 = (s1 - s0).cross(s2 - s0);

        check_flag_mac0(&mut self.flag, self.mac0);
    }

    /// Cross product of two vectors
    pub fn op(&mut self, cmd: GteCommand) {
        debug!("gte command, op");

        let sf = cmd.sf();
        let lm = cmd.lm();

        let d = Vector3 {
            x: self.rtm.at(0, 0) as i64,
            y: self.rtm.at(1, 1) as i64,
            z: self.rtm.at(2, 2) as i64,
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

    /// Depth cueing (single)
    pub fn dpcs(&mut self, cmd: GteCommand) {
        debug!("gte command, dpcs");

        let sf = cmd.sf();
        let lm = cmd.lm();

        let rgbc = self.rgbc;

        let rgb_vec = Vector3 {
            x: rgbc.r as i64,
            y: rgbc.g as i64,
            z: rgbc.b as i64,
        };

        let fc = Vector3 {
            x: self.fc.x as i64,
            y: self.fc.y as i64,
            z: self.fc.z as i64,
        } << 12;

        let mac = rgb_vec << 16;
        let mac = mac + (fc - mac) * (self.ir0 as i64);
        self.macv = mac >> (sf * 12);

        self.colors.push(Color {
            c: rgbc.c,
            r: (self.macv.x >> 4) as u8,
            g: (self.macv.y >> 4) as u8,
            b: (self.macv.z >> 4) as u8,
        });

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
