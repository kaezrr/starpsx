use crate::consts::{
    AVG_1ST_RESP, AVG_2ND_RESP_GET_ID, AVG_2ND_RESP_SEEKL, CDROM_VERSION, GET_ID_RESPONSE,
};
use tracing::debug;

use super::*;

#[derive(PartialEq, Clone)]
pub enum ResponseType {
    INT3(Vec<u8>),
    INT2(Vec<u8>),
    INT2Seek,
}

#[derive(Default)]
pub struct CommandResponse {
    responses: ArrayVec<(ResponseType, u64), 2>,
}

impl CommandResponse {
    pub fn push(&mut self, res_type: ResponseType, delay: u64) {
        self.responses.push((res_type, delay));
    }

    pub fn get(self) -> ArrayVec<(ResponseType, u64), 2> {
        self.responses
    }
}

impl CdRom {
    pub fn test(&mut self) -> CommandResponse {
        let cmd = self.parameters[0];
        let mut responses = CommandResponse::default();
        match cmd {
            0x20 => {
                debug!(subcmd = "get version", "cdrom test command");

                responses.push(
                    ResponseType::INT3(CDROM_VERSION.into()),
                    self.speed.transform(AVG_1ST_RESP),
                );
            }
            _ => unimplemented!("cdrom command Test {cmd:02x}"),
        };
        responses
    }

    pub fn nop(&mut self) -> CommandResponse {
        debug!(status=?self.status.0, "cdrom nop command");

        let mut responses = CommandResponse::default();
        responses.push(
            ResponseType::INT3(vec![self.status.0]),
            self.speed.transform(AVG_1ST_RESP),
        );
        responses
    }

    pub fn get_id(&mut self) -> CommandResponse {
        debug!("cdrom get id");

        let mut responses = CommandResponse::default();
        responses.push(
            ResponseType::INT3(vec![self.status.0]),
            self.speed.transform(AVG_1ST_RESP),
        );
        responses.push(
            ResponseType::INT2(GET_ID_RESPONSE.into()),
            self.speed.transform(AVG_2ND_RESP_GET_ID),
        );
        responses
    }

    pub fn set_loc(&mut self) -> CommandResponse {
        debug!(?self.parameters, "cdrom set loc");

        let mins = bcd_to_u8(self.parameters[0]).unwrap();
        let secs = bcd_to_u8(self.parameters[1]).unwrap();
        let sect = bcd_to_u8(self.parameters[2]).unwrap();

        self.disc
            .as_mut()
            .expect("set loc while no disk inserted")
            .seek_location(mins, secs, sect);

        let mut responses = CommandResponse::default();
        responses.push(ResponseType::INT3(vec![self.status.0]), AVG_1ST_RESP);
        responses
    }

    pub fn seekl(&mut self) -> CommandResponse {
        debug!("cdrom seekl");

        self.status.set_seeking(true);

        let mut responses = CommandResponse::default();
        responses.push(
            ResponseType::INT3(vec![self.status.0]),
            self.speed.transform(AVG_1ST_RESP),
        );
        responses.push(
            ResponseType::INT2Seek,
            self.speed.transform(AVG_2ND_RESP_SEEKL),
        );
        responses
    }

    pub fn setmode(&mut self) -> CommandResponse {
        debug!(?self.parameters, "cdrom set mode");

        let mode = self.parameters[0];

        self.speed = match mode & (1 << 7) == 0 {
            true => Speed::Double,
            false => Speed::Normal,
        };

        self.sector_size = match mode & (1 << 5) == 0 {
            true => SectorSize::DataOnly,
            false => SectorSize::WholeSectorExceptSyncBytes,
        };

        let mut responses = CommandResponse::default();
        responses.push(
            ResponseType::INT3(vec![self.status.0]),
            self.speed.transform(AVG_1ST_RESP),
        );
        responses
    }
}

fn bcd_to_u8(bcd_val: u8) -> Option<u8> {
    let hi = (bcd_val >> 4) & 0xF;
    let lo = bcd_val & 0xF;

    if hi < 10 && lo < 10 {
        Some(hi * 10 + lo)
    } else {
        None
    }
}
