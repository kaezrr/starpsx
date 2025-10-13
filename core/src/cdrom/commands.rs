use super::*;

pub enum Response {
    INT3(ArrayVec<u8, 4>),
}

impl CdRom {
    pub fn test(&mut self, cmd: u8) -> Response {
        let mut result = ArrayVec::default();
        match cmd {
            // CDROM Version
            0x20 => {
                result.push(0x95);
                result.push(0x05);
                result.push(0x16);
                result.push(0xC1);
                Response::INT3(result)
            }
            _ => unimplemented!("cdrom command Test {cmd:02x}"),
        }
    }

    pub fn nop(&mut self) -> Response {
        let mut result = ArrayVec::default();
        result.push(0x02);
        Response::INT3(result)
    }
}
