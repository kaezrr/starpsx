use super::*;

impl GTEngine {
    /// Perspective transformation(single)
    pub fn rtps(&mut self, cmd: GteCommand) {
        debug!("gte command, rtpt");

        let sf = cmd.sf();
        let lm = SaturationRange::Signed16;

        self.macv = (self.tr * 0x1000 + &self.rtm * self.v[0]) >> (sf * 12);
        self.ir = Vector3::<i32>::from(self.macv).saturated(lm, true).into();
    }

    /// Perspective transformation(triple)
    pub fn rtpt(&mut self) {
        debug!("gte command, rtpt");
    }

    /// Normal color depth cue single vector
    pub fn ncds(&mut self) {
        debug!("gte command, ncds");
    }

    /// Normal clipping
    pub fn nclip(&mut self) {
        debug!("gte command, nclip");
    }
}
