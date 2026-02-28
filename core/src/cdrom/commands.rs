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
        if self.parameters.len() != 1 {
            return error_response(&mut self.status, 0x20);
        }

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

            0x60 => {
                debug!(target: "cdrom", subcmd = "read one byte from drive ram or i/o", "cdrom test command");

                responses.push(ResponseType::INT3(vec![0]), AVG_1ST_RESP_GENERIC);
            }

            _ => unimplemented!("cdrom command Test {cmd:02x}"),
        };
        responses
    }

    pub fn nop(&mut self) -> CommandResponse {
        if !self.parameters.is_empty() {
            return error_response(&mut self.status, 0x20);
        }

        debug!(target: "cdrom", status=?self.status.0, "cdrom nop command");

        let mut responses = CommandResponse::default();
        responses.push(
            ResponseType::INT3(vec![self.status.0]),
            AVG_1ST_RESP_GENERIC,
        );
        responses
    }

    pub fn get_id(&mut self) -> CommandResponse {
        if !self.parameters.is_empty() {
            return error_response(&mut self.status, 0x20);
        }

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
        if self.parameters.len() != 3 {
            return error_response(&mut self.status, 0x20);
        }

        debug!(target: "cdrom", params=?self.parameters, "cdrom set loc");

        let mins_res = from_bcd(self.parameters[0]);
        let secs_res = from_bcd(self.parameters[1]);
        let sect_res = from_bcd(self.parameters[2]);

        let mut responses = CommandResponse::default();
        if let (Some(mins), Some(secs), Some(sect)) = (mins_res, secs_res, sect_res) {
            self.disk
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
        if !self.parameters.is_empty() {
            return error_response(&mut self.status, 0x20);
        }

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
        if self.parameters.len() != 1 {
            return error_response(&mut self.status, 0x20);
        }

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
        if !self.parameters.is_empty() {
            return error_response(&mut self.status, 0x20);
        }

        debug!(target: "cdrom", "cdrom reads");

        self.readn()
    }

    pub fn readn(&mut self) -> CommandResponse {
        if !self.parameters.is_empty() {
            return error_response(&mut self.status, 0x20);
        }

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
        if !self.parameters.is_empty() {
            return error_response(&mut self.status, 0x20);
        }

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
        if !self.parameters.is_empty() {
            return error_response(&mut self.status, 0x20);
        }

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
        if !self.parameters.is_empty() {
            return error_response(&mut self.status, 0x20);
        }

        debug!(target: "cdrom", "cdrom set filter");

        let mut responses = CommandResponse::default();
        responses.push(ResponseType::INT3(vec![self.status.0]), AVG_1ST_RESP_INIT);

        responses
    }

    pub fn play(&mut self) -> CommandResponse {
        if !self.parameters.is_empty() {
            return error_response(&mut self.status, 0x20);
        }

        debug!(target: "cdrom", "cdrom play");

        let mut responses = CommandResponse::default();
        responses.push(ResponseType::INT3(vec![self.status.0]), AVG_1ST_RESP_INIT);

        responses
    }

    // stubbed audio command
    pub fn demute(&mut self) -> CommandResponse {
        if !self.parameters.is_empty() {
            return error_response(&mut self.status, 0x20);
        }

        debug!(target: "cdrom", "cdrom demute");

        let mut responses = CommandResponse::default();
        responses.push(ResponseType::INT3(vec![self.status.0]), AVG_1ST_RESP_INIT);

        responses
    }

    pub fn get_tn(&mut self) -> CommandResponse {
        if !self.parameters.is_empty() {
            return error_response(&mut self.status, 0x20);
        }

        debug!(target: "cdrom", "cdrom get tn");

        let disk = &self.disk.as_ref().unwrap();

        let first_track = to_bcd(disk.first_track_id()).unwrap();
        let last_track = to_bcd(disk.last_track_id()).unwrap();

        let mut responses = CommandResponse::default();
        responses.push(
            ResponseType::INT3(vec![self.status.0, first_track, last_track]),
            AVG_1ST_RESP_INIT,
        );

        responses
    }

    pub fn get_td(&mut self) -> CommandResponse {
        if self.parameters.len() != 1 {
            return error_response(&mut self.status, 0x20);
        }

        debug!(target: "cdrom", "cdrom get td");

        let disk = &self.disk.as_ref().unwrap();
        let last_track = disk.last_track_id();

        let Some(track) = from_bcd(self.parameters[0]).filter(|&x| x <= last_track) else {
            let mut response = CommandResponse::default();

            self.status.set_error(true);
            response.push(ResponseType::INT5([self.status.0, 0x10]), AVG_1ST_RESP_INIT);
            self.status.set_error(false);

            return response;
        };

        let (mm, ss, _) = if track != 0 {
            disk.track_mm_ss_ff(track)
        } else {
            disk.last_track_end()
        };

        let mut responses = CommandResponse::default();
        responses.push(
            ResponseType::INT3(vec![
                self.status.0,
                to_bcd(mm).unwrap(),
                to_bcd(ss).unwrap(),
            ]),
            AVG_1ST_RESP_INIT,
        );

        responses
    }

    pub fn stop(&mut self) -> CommandResponse {
        if !self.parameters.is_empty() {
            return error_response(&mut self.status, 0x20);
        }

        debug!(target: "cdrom", "cdrom stop");

        let mut responses = CommandResponse::default();

        self.status.set_reading(false);
        responses.push(ResponseType::INT3(vec![self.status.0]), AVG_1ST_RESP_INIT);

        if let Some(cd) = self.disk.as_mut() {
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

    pub fn get_locp(&mut self) -> CommandResponse {
        if !self.parameters.is_empty() {
            return error_response(&mut self.status, 0x20);
        }

        debug!(target: "cdrom", "cdrom getlocp");

        let mut responses = CommandResponse::default();
        responses.push(
            ResponseType::INT3(vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
            AVG_1ST_RESP_GENERIC,
        );

        responses
    }
}

fn error_response(stat: &mut Status, err_byte: u8) -> CommandResponse {
    let mut response = CommandResponse::default();

    stat.set_error(true);
    response.push(ResponseType::INT5([stat.0, err_byte]), AVG_1ST_RESP_INIT);
    stat.set_error(false);

    response
}

fn from_bcd(bcd: u8) -> Option<u8> {
    let tens = bcd >> 4;
    let ones = bcd & 0x0F;

    if tens <= 9 && ones <= 9 {
        Some(tens * 10 + ones)
    } else {
        None
    }
}

fn to_bcd(val: u8) -> Option<u8> {
    if val > 99 {
        return None;
    }

    let tens = val / 10;
    let ones = val % 10;

    Some((tens << 4) | ones)
}
