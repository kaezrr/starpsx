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
    }

    /// Cross product of two vectors
    pub fn op(&mut self, cmd: GteCommand) {
        debug!("gte command, op");
    }

    /// Depth cueing (single)
    pub fn dpcs(&mut self, cmd: GteCommand) {
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
