use crate::consts::{
    AVG_1ST_RESP_GENERIC, AVG_1ST_RESP_INIT, AVG_2ND_RESP_GET_ID, AVG_2ND_RESP_PAUSE,
    AVG_2ND_RESP_SEEKL, AVG_RATE_INT1, CDROM_VERSION, GET_ID_RESPONSE,
};
use tracing::trace;

use super::*;

#[derive(PartialEq, Clone)]
pub enum ResponseType {
    INT3(Vec<u8>),
    INT2(Vec<u8>),
    INT2Seek,
    INT1Stat,
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
                trace!(subcmd = "get version", "cdrom test command");

                responses.push(
                    ResponseType::INT3(CDROM_VERSION.into()),
                    self.speed.transform(AVG_1ST_RESP_GENERIC),
                );
            }
            _ => unimplemented!("cdrom command Test {cmd:02x}"),
        };
        responses
    }

    pub fn nop(&mut self) -> CommandResponse {
        trace!(status=?self.status.0, "cdrom nop command");

        let mut responses = CommandResponse::default();
        responses.push(
            ResponseType::INT3(vec![self.status.0]),
            self.speed.transform(AVG_1ST_RESP_GENERIC),
        );
        responses
    }

    pub fn get_id(&mut self) -> CommandResponse {
        trace!("cdrom get id");

        let mut responses = CommandResponse::default();
        responses.push(
            ResponseType::INT3(vec![self.status.0]),
            self.speed.transform(AVG_1ST_RESP_GENERIC),
        );
        responses.push(
            ResponseType::INT2(GET_ID_RESPONSE.into()),
            self.speed
                .transform(AVG_1ST_RESP_GENERIC + AVG_2ND_RESP_GET_ID),
        );
        responses
    }

    pub fn set_loc(&mut self) -> CommandResponse {
        trace!(?self.parameters, "cdrom set loc");

        let mins = bcd_to_u8(self.parameters[0]).unwrap();
        let secs = bcd_to_u8(self.parameters[1]).unwrap();
        let sect = bcd_to_u8(self.parameters[2]).unwrap();

        self.disc
            .as_mut()
            .expect("set loc while no disk inserted")
            .seek_location(mins, secs, sect);

        let mut responses = CommandResponse::default();
        responses.push(
            ResponseType::INT3(vec![self.status.0]),
            self.speed.transform(AVG_1ST_RESP_GENERIC),
        );
        responses
    }

    pub fn seekl(&mut self) -> CommandResponse {
        trace!("cdrom seekl");

        self.status.set_seeking(true);

        let mut responses = CommandResponse::default();
        responses.push(
            ResponseType::INT3(vec![self.status.0]),
            self.speed.transform(AVG_1ST_RESP_GENERIC),
        );
        responses.push(
            ResponseType::INT2Seek,
            self.speed
                .transform(AVG_1ST_RESP_GENERIC + AVG_2ND_RESP_SEEKL),
        );
        responses
    }

    pub fn setmode(&mut self) -> CommandResponse {
        trace!(?self.parameters, "cdrom set mode");

        let mode = self.parameters[0];

        self.speed = match mode & (1 << 7) != 0 {
            true => Speed::Double,
            false => Speed::Normal,
        };

        // Set sector size only if ignore bit is 0
        if mode & (1 << 4) == 0 {
            self.sector_size = match mode & (1 << 5) != 0 {
                true => SectorSize::WholeSectorExceptSyncBytes,
                false => SectorSize::DataOnly,
            };
        }

        let mut responses = CommandResponse::default();
        responses.push(
            ResponseType::INT3(vec![self.status.0]),
            self.speed.transform(AVG_1ST_RESP_GENERIC),
        );
        responses
    }

    pub fn readn(&mut self) -> CommandResponse {
        trace!("cdrom readn");

        self.status.set_reading(true);

        let mut responses = CommandResponse::default();
        responses.push(
            ResponseType::INT3(vec![self.status.0]),
            self.speed.transform(AVG_1ST_RESP_GENERIC),
        );
        responses.push(
            ResponseType::INT1Stat,
            self.speed.transform(AVG_1ST_RESP_GENERIC + AVG_RATE_INT1),
        );
        responses
    }

    pub fn pause(&mut self) -> CommandResponse {
        trace!("cdrom pause");

        let mut responses = CommandResponse::default();
        responses.push(
            ResponseType::INT3(vec![self.status.0]),
            self.speed.transform(AVG_1ST_RESP_GENERIC),
        );

        self.status.set_reading(false);
        responses.push(
            ResponseType::INT2(vec![self.status.0]),
            self.speed
                .transform(AVG_1ST_RESP_GENERIC + AVG_2ND_RESP_PAUSE),
        );

        responses
    }

    pub fn init(&mut self) -> CommandResponse {
        trace!("cdrom init");

        self.speed = Speed::Normal;
        self.sector_size = SectorSize::WholeSectorExceptSyncBytes;

        let mut responses = CommandResponse::default();
        responses.push(
            ResponseType::INT3(vec![self.status.0]),
            self.speed.transform(AVG_1ST_RESP_INIT),
        );

        self.status.set_motor_on(true);

        responses.push(
            ResponseType::INT2(vec![self.status.0]),
            self.speed
                .transform(AVG_1ST_RESP_GENERIC + AVG_2ND_RESP_SEEKL),
        );

        responses
    }

    pub fn demute(&mut self) -> CommandResponse {
        trace!("cdrom demute");

        let mut responses = CommandResponse::default();
        responses.push(ResponseType::INT3(vec![self.status.0]), AVG_1ST_RESP_INIT);

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
