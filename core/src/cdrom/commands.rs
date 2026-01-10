use crate::consts::{
    AVG_1ST_RESP_GENERIC, AVG_1ST_RESP_INIT, AVG_2ND_RESP_GET_ID, AVG_2ND_RESP_PAUSE,
    AVG_2ND_RESP_SEEKL, AVG_RATE_INT1, CDROM_VERSION, GET_ID_RESPONSE,
};
use tracing::{debug, error};

use super::*;

#[derive(PartialEq, Clone)]
pub enum ResponseType {
    INT3(Vec<u8>),
    INT2(Vec<u8>),
    INT5([u8; 2]),
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
                debug!(target: "cdrom", subcmd = "get version", "cdrom test command");

                responses.push(
                    ResponseType::INT3(CDROM_VERSION.into()),
                    AVG_1ST_RESP_GENERIC,
                );
            }
            _ => unimplemented!("cdrom command Test {cmd:02x}"),
        };
        responses
    }

    pub fn nop(&mut self) -> CommandResponse {
        debug!(target: "cdrom", status=?self.status.0, "cdrom nop command");

        let mut responses = CommandResponse::default();
        responses.push(
            ResponseType::INT3(vec![self.status.0]),
            AVG_1ST_RESP_GENERIC,
        );
        responses
    }

    pub fn get_id(&mut self) -> CommandResponse {
        debug!(target: "cdrom", "cdrom get id");

        let mut responses = CommandResponse::default();
        responses.push(
            ResponseType::INT3(vec![self.status.0]),
            AVG_1ST_RESP_GENERIC,
        );
        responses.push(
            ResponseType::INT2(GET_ID_RESPONSE.into()),
            AVG_1ST_RESP_GENERIC + AVG_2ND_RESP_GET_ID,
        );
        responses
    }

    // TODO: better error handling with result
    pub fn set_loc(&mut self) -> CommandResponse {
        debug!(target: "cdrom", params=?self.parameters, "cdrom set loc");

        let mins_res = bcd_to_u8(self.parameters[0]);
        let secs_res = bcd_to_u8(self.parameters[1]);
        let sect_res = bcd_to_u8(self.parameters[2]);

        let mut responses = CommandResponse::default();
        if let (Some(mins), Some(secs), Some(sect)) = (mins_res, secs_res, sect_res) {
            self.disc
                .as_mut()
                .expect("set loc while no disk inserted")
                .seek_location(mins, secs, sect);

            responses.push(
                ResponseType::INT3(vec![self.status.0]),
                AVG_1ST_RESP_GENERIC,
            );
        } else {
            error!(
                "invalid/out of range seek to {:2X}:{:2X}:{:2X}",
                self.parameters[0], self.parameters[1], self.parameters[2]
            );

            self.status.set_error(true);
            responses.push(
                ResponseType::INT5([self.status.0, 0x10]),
                AVG_1ST_RESP_GENERIC,
            );
            self.status.set_error(false);
        }

        responses
    }

    pub fn seekl(&mut self) -> CommandResponse {
        debug!(target: "cdrom", "cdrom seekl");

        self.status.set_seeking(true);

        let mut responses = CommandResponse::default();
        responses.push(
            ResponseType::INT3(vec![self.status.0]),
            AVG_1ST_RESP_GENERIC,
        );
        responses.push(
            ResponseType::INT2Seek,
            AVG_1ST_RESP_GENERIC + AVG_2ND_RESP_SEEKL,
        );
        responses
    }

    pub fn setmode(&mut self) -> CommandResponse {
        debug!(target: "cdrom", params=?self.parameters, "cdrom set mode");

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
            AVG_1ST_RESP_GENERIC,
        );
        responses
    }

    pub fn reads(&mut self) -> CommandResponse {
        debug!(target: "cdrom", "cdrom reads");

        self.readn()
    }

    pub fn readn(&mut self) -> CommandResponse {
        debug!(target: "cdrom", "cdrom readn");

        self.status.set_reading(true);

        let mut responses = CommandResponse::default();
        responses.push(
            ResponseType::INT3(vec![self.status.0]),
            AVG_1ST_RESP_GENERIC,
        );

        responses.push(
            ResponseType::INT1Stat,
            AVG_1ST_RESP_GENERIC + self.speed.transform(AVG_RATE_INT1),
        );
        responses
    }

    pub fn pause(&mut self) -> CommandResponse {
        debug!(target: "cdrom", "cdrom pause");

        let mut responses = CommandResponse::default();
        responses.push(
            ResponseType::INT3(vec![self.status.0]),
            AVG_1ST_RESP_GENERIC,
        );

        self.status.set_reading(false);

        responses.push(
            ResponseType::INT2(vec![self.status.0]),
            AVG_1ST_RESP_GENERIC + self.speed.transform(AVG_2ND_RESP_PAUSE),
        );

        responses
    }

    pub fn init(&mut self) -> CommandResponse {
        debug!(target: "cdrom", "cdrom init");

        self.speed = Speed::Normal;
        self.sector_size = SectorSize::WholeSectorExceptSyncBytes;

        let mut responses = CommandResponse::default();
        responses.push(ResponseType::INT3(vec![self.status.0]), AVG_1ST_RESP_INIT);

        self.status.set_motor_on(true);

        responses.push(
            ResponseType::INT2(vec![self.status.0]),
            AVG_1ST_RESP_GENERIC + AVG_2ND_RESP_SEEKL,
        );

        responses
    }

    // stubbed audio command
    pub fn set_filter(&mut self) -> CommandResponse {
        debug!(target: "cdrom", "cdrom set filter");

        let mut responses = CommandResponse::default();
        responses.push(ResponseType::INT3(vec![self.status.0]), AVG_1ST_RESP_INIT);

        responses
    }

    pub fn play(&mut self) -> CommandResponse {
        debug!(target: "cdrom", "cdrom play");

        let mut responses = CommandResponse::default();
        responses.push(ResponseType::INT3(vec![self.status.0]), AVG_1ST_RESP_INIT);

        responses
    }

    // stubbed audio command
    pub fn demute(&mut self) -> CommandResponse {
        debug!(target: "cdrom", "cdrom demute");

        let mut responses = CommandResponse::default();
        responses.push(ResponseType::INT3(vec![self.status.0]), AVG_1ST_RESP_INIT);

        responses
    }

    // stub values, need to load cue sheet
    pub fn get_tn(&mut self) -> CommandResponse {
        debug!(target: "cdrom", "cdrom get tn");

        let mut responses = CommandResponse::default();
        responses.push(
            ResponseType::INT3(vec![self.status.0, 1, 1]),
            AVG_1ST_RESP_INIT,
        );

        responses
    }

    // stub values, need to load cue sheet
    pub fn get_td(&mut self) -> CommandResponse {
        debug!(target: "cdrom", "cdrom get tn");

        let mut responses = CommandResponse::default();
        responses.push(
            ResponseType::INT3(vec![self.status.0, 1, 1]),
            AVG_1ST_RESP_INIT,
        );

        responses
    }

    pub fn stop(&mut self) -> CommandResponse {
        debug!(target: "cdrom", "cdrom stop");

        let mut responses = CommandResponse::default();

        self.status.set_reading(false);
        responses.push(ResponseType::INT3(vec![self.status.0]), AVG_1ST_RESP_INIT);

        if let Some(cd) = self.disc.as_mut() {
            cd.seek_location(0, 2, 0)
        }

        self.status.set_motor_on(false);

        responses.push(
            ResponseType::INT2(vec![self.status.0]),
            match self.speed {
                Speed::Normal => 0x0d38aca,
                Speed::Double => 0x18a6076,
            },
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
