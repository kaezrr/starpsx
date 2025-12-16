use tracing::debug;

use super::*;

pub enum Response {
    INT3(Vec<u8>),
}

impl CdRom {
    pub fn test(&mut self, cmd: u8) -> Response {
        match cmd {
            0x20 => {
                debug!(subcmd = "get version", "cdrom test command");
                Response::INT3(vec![0x95, 0x05, 0x16, 0xC1])
            }
            _ => unimplemented!("cdrom command Test {cmd:02x}"),
        }
    }

    pub fn nop(&mut self) -> Response {
        debug!(status=?self.status.0, "cdrom nop command");
        Response::INT3(vec![self.status.0])
    }
}
