use arrayvec::ArrayVec;

use crate::consts::{
    AVG_1ST_RESP_GENERIC, AVG_1ST_RESP_INIT, AVG_2ND_RESP_GET_ID, AVG_2ND_RESP_PAUSE,
    AVG_2ND_RESP_SEEKL, AVG_RATE_INT1,
};

use tracing::{debug, error};

use super::*;

impl CdRom {
    pub fn test(&mut self) -> CommandResponse {
        if self.parameters.len() != 1 {
            return error_response(&self.status, 0x20);
        }

        let cmd = self.parameters[0];
        match cmd {
            0x20 => {
                debug!(target: "cdrom", subcmd = "get version", "cdrom test command");

                CommandResponse::new().int3([149, 5, 22, 193], AVG_1ST_RESP_GENERIC)
            }

            0x60 => {
                debug!(target: "cdrom", subcmd = "read one byte from drive ram or i/o", "cdrom test command");

                CommandResponse::new().int3([0], AVG_1ST_RESP_GENERIC)
            }

            _ => unimplemented!("cdrom command Test {cmd:02x}"),
        }
    }

    pub fn nop(&mut self) -> CommandResponse {
        if !self.parameters.is_empty() {
            return error_response(&self.status, 0x20);
        }

        debug!(target: "cdrom", status=?self.status.0, "cdrom nop command");

        CommandResponse::new().int3([self.status.0], AVG_1ST_RESP_GENERIC)
    }

    pub fn get_id(&mut self) -> CommandResponse {
        if !self.parameters.is_empty() {
            return error_response(&self.status, 0x20);
        }

        debug!(target: "cdrom", "cdrom get id");

        CommandResponse::new()
            .int3([self.status.0], AVG_1ST_RESP_GENERIC)
            .int2(
                [0x02, 0x00, 0x20, 0x00, b'S', b'C', b'E', b'A'],
                AVG_1ST_RESP_GENERIC + AVG_2ND_RESP_GET_ID,
            )
    }

    pub fn set_loc(&mut self) -> CommandResponse {
        if self.parameters.len() != 3 {
            return error_response(&self.status, 0x20);
        }

        debug!(target: "cdrom", params=?self.parameters, "cdrom set loc");

        let mm = self.parameters[0];
        let ss = self.parameters[1];
        let ff = self.parameters[2];

        let (Some(m), Some(s), Some(f)) = (from_bcd(mm), from_bcd(ss), from_bcd(ff)) else {
            error!("invalid/out of range seek to {mm:2X}:{ss:2X}:{ff:2X}",);
            return CommandResponse::new()
                .int5([self.status.with_error(), 0x10], AVG_1ST_RESP_GENERIC);
        };

        self.disk
            .as_mut()
            .expect("set loc while no disk inserted")
            .seek_location(m, s, f);

        CommandResponse::new().int3([self.status.0], AVG_1ST_RESP_GENERIC)
    }

    pub fn seekl(&mut self) -> CommandResponse {
        if !self.parameters.is_empty() {
            return error_response(&self.status, 0x20);
        }

        debug!(target: "cdrom", "cdrom seekl");

        CommandResponse::new()
            .int3([self.status.0], AVG_1ST_RESP_GENERIC)
            .int2([self.status.0], AVG_1ST_RESP_GENERIC + AVG_2ND_RESP_SEEKL)
    }

    pub fn seekp(&mut self) -> CommandResponse {
        if !self.parameters.is_empty() {
            return error_response(&self.status, 0x20);
        }

        debug!(target: "cdrom", "cdrom seekp");

        CommandResponse::new()
            .int3([self.status.0], AVG_1ST_RESP_GENERIC)
            .int2([self.status.0], AVG_1ST_RESP_GENERIC + AVG_2ND_RESP_SEEKL)
    }

    pub fn setmode(&mut self) -> CommandResponse {
        if self.parameters.len() != 1 {
            return error_response(&self.status, 0x20);
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

        CommandResponse::new().int3([self.status.0], AVG_1ST_RESP_GENERIC)
    }

    pub fn reads(&mut self) -> CommandResponse {
        if !self.parameters.is_empty() {
            return error_response(&self.status, 0x20);
        }

        debug!(target: "cdrom", "cdrom reads");

        self.readn()
    }

    pub fn readn(&mut self) -> CommandResponse {
        if !self.parameters.is_empty() {
            return error_response(&self.status, 0x20);
        }

        debug!(target: "cdrom", "cdrom readn");

        self.status.set_reading(true);

        CommandResponse::new()
            .int3([self.status.0], AVG_1ST_RESP_GENERIC)
            .int1(AVG_1ST_RESP_GENERIC + self.speed.transform(AVG_RATE_INT1))
    }

    pub fn pause(&mut self) -> CommandResponse {
        if !self.parameters.is_empty() {
            return error_response(&self.status, 0x20);
        }

        debug!(target: "cdrom", "cdrom pause");

        let before = self.status.clear_reading();

        CommandResponse::new()
            .int3([before], AVG_1ST_RESP_GENERIC)
            .int2(
                [self.status.0],
                AVG_1ST_RESP_GENERIC + self.speed.transform(AVG_2ND_RESP_PAUSE),
            )
    }

    pub fn init(&mut self) -> CommandResponse {
        if !self.parameters.is_empty() {
            return error_response(&self.status, 0x20);
        }

        debug!(target: "cdrom", "cdrom init");

        self.speed = Speed::Normal;
        self.sector_size = SectorSize::WholeSectorExceptSyncBytes;

        if let Some(disk) = self.disk.as_mut() {
            disk.reset_read_head();
        }

        let before = self.status.enable_motor();

        CommandResponse::new()
            .int3([before], AVG_1ST_RESP_INIT)
            .int2([self.status.0], AVG_1ST_RESP_GENERIC + AVG_2ND_RESP_SEEKL)
    }

    // stubbed audio command
    pub fn set_filter(&mut self) -> CommandResponse {
        if !self.parameters.is_empty() {
            return error_response(&self.status, 0x20);
        }

        debug!(target: "cdrom", "cdrom set filter");

        CommandResponse::new().int3([self.status.0], AVG_1ST_RESP_INIT)
    }

    pub fn play(&mut self) -> CommandResponse {
        debug!(target: "cdrom", "cdrom play");

        CommandResponse::new().int3([self.status.0], AVG_1ST_RESP_INIT)
    }

    // stubbed audio command
    pub fn demute(&mut self) -> CommandResponse {
        if !self.parameters.is_empty() {
            return error_response(&self.status, 0x20);
        }

        debug!(target: "cdrom", "cdrom demute");

        CommandResponse::new().int3([self.status.0], AVG_1ST_RESP_INIT)
    }

    pub fn get_tn(&mut self) -> CommandResponse {
        if !self.parameters.is_empty() {
            return error_response(&self.status, 0x20);
        }

        debug!(target: "cdrom", "cdrom get tn");

        let disk = &self.disk.as_ref().unwrap();

        let first_track = to_bcd(disk.first_track_id()).unwrap();
        let last_track = to_bcd(disk.last_track_id()).unwrap();

        CommandResponse::new().int3([self.status.0, first_track, last_track], AVG_1ST_RESP_INIT)
    }

    pub fn get_td(&mut self) -> CommandResponse {
        if self.parameters.len() != 1 {
            return error_response(&self.status, 0x20);
        }

        debug!(target: "cdrom", "cdrom get td");

        let disk = &self.disk.as_ref().unwrap();
        let last_track = disk.last_track_id();

        let Some(track) = from_bcd(self.parameters[0]).filter(|&x| x <= last_track) else {
            return CommandResponse::new()
                .int5([self.status.with_error(), 0x10], AVG_1ST_RESP_INIT);
        };

        let (mm, ss, _) = if track != 0 {
            disk.track_mm_ss_ff(track)
        } else {
            disk.last_track_end()
        };

        CommandResponse::new().int3(
            [self.status.0, to_bcd(mm).unwrap(), to_bcd(ss).unwrap()],
            AVG_1ST_RESP_INIT,
        )
    }

    pub fn stop(&mut self) -> CommandResponse {
        if !self.parameters.is_empty() {
            return error_response(&self.status, 0x20);
        }

        debug!(target: "cdrom", "cdrom stop");

        self.status.clear_reading();
        let after_reading = self.status.0;

        if let Some(cd) = self.disk.as_mut() {
            cd.seek_location(0, 2, 0)
        }

        self.status.disable_motor();

        let delay = match self.speed {
            Speed::Normal => 0x0d38aca,
            Speed::Double => 0x18a6076,
        };

        CommandResponse::new()
            .int3([after_reading], AVG_1ST_RESP_INIT)
            .int2([self.status.0], delay)
    }

    pub fn get_locp(&mut self) -> CommandResponse {
        if !self.parameters.is_empty() {
            return error_response(&self.status, 0x20);
        }

        debug!(target: "cdrom", "cdrom getlocp");

        let disk = self.disk.as_ref().unwrap();

        CommandResponse::new().int3(
            disk.position_info().map(|x| to_bcd(x).unwrap()),
            AVG_1ST_RESP_GENERIC,
        )
    }
}

fn error_response(stat: &Status, err_byte: u8) -> CommandResponse {
    CommandResponse::new().int5([stat.with_error(), err_byte], AVG_1ST_RESP_INIT)
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

#[derive(PartialEq, Clone)]
pub enum ResponseType {
    INT3(ArrayVec<u8, 8>),
    INT2(ArrayVec<u8, 8>),
    INT5([u8; 2]),
    INT1,
}

#[derive(Default)]
pub struct CommandResponse {
    pub responses: ArrayVec<(ResponseType, u64), 2>,
}

impl CommandResponse {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn int3<const N: usize>(mut self, data: [u8; N], delay: u64) -> Self {
        let arr = ArrayVec::from_iter(data);
        self.responses.push((ResponseType::INT3(arr), delay));
        self
    }

    pub fn int2<const N: usize>(mut self, data: [u8; N], delay: u64) -> Self {
        let arr = ArrayVec::from_iter(data);
        self.responses.push((ResponseType::INT2(arr), delay));
        self
    }

    pub fn int5(mut self, data: [u8; 2], delay: u64) -> Self {
        self.responses.push((ResponseType::INT5(data), delay));
        self
    }

    pub fn int1(mut self, delay: u64) -> Self {
        self.responses.push((ResponseType::INT1, delay));
        self
    }
}
