use super::*;

impl GTEngine {
    /// Perspective transformation(single)
    pub fn rtps(&mut self, cmd: GteCommand) {
        debug!("gte command, rtpt");
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
