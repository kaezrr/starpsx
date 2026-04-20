use arrayvec::ArrayVec;
use tracing::debug;
use tracing::error;

use super::CdRom;
use super::Speed;
use super::Status;
use crate::consts::AVG_1ST_RESP_GENERIC;
use crate::consts::AVG_1ST_RESP_INIT;
use crate::consts::AVG_2ND_RESP_GET_ID;
use crate::consts::AVG_2ND_RESP_PAUSE;
use crate::consts::AVG_2ND_RESP_SEEKL;
use crate::consts::AVG_RATE_INT1;

impl CdRom {
    pub fn test(&self) -> CommandResponse {
        if self.parameters.len() != 1 {
            return error_response(&self.status, 0x20, "test expects 1 parameter");
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

            0x77..=0xFF => error_response(&self.status, 0x10, "unknown test command"),

            _ => unimplemented!("cdrom command Test {cmd:02x}"),
        }
    }

    pub fn invalid(&self) -> CommandResponse {
        error_response(&self.status, 0x40, "invalid command")
    }

    pub fn nop(&self) -> CommandResponse {
        if !self.parameters.is_empty() {
            return error_response(&self.status, 0x20, "nop takes no parameters");
        }

        debug!(target: "cdrom", status=?self.status.0, "cdrom nop command");

        CommandResponse::new().int3([self.status.0], AVG_1ST_RESP_GENERIC)
    }

    pub fn get_id(&self) -> CommandResponse {
        if !self.parameters.is_empty() {
            return error_response(&self.status, 0x20, "get_id takes no parameters");
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
            return error_response(&self.status, 0x20, "set_loc expects 3 parameters");
        }

        debug!(target: "cdrom", params=?self.parameters, "cdrom set loc");

        let mm = self.parameters[0];
        let ss = self.parameters[1];
        let ff = self.parameters[2];

        let Some((m, s, f)) = validate_seek(mm, ss, ff) else {
            error!("invalid/out of range seek to {mm:02x}:{ss:02x}:{ff:02x}",);
            return CommandResponse::new()
                .int5([self.status.with_error(), 0x10], AVG_1ST_RESP_GENERIC);
        };

        self.disk
            .as_mut()
            .expect("set_loc inserted disk")
            .seek_location(m, s, f);

        CommandResponse::new().int3([self.status.0], AVG_1ST_RESP_GENERIC)
    }

    pub fn seekl(&mut self) -> CommandResponse {
        if !self.parameters.is_empty() {
            return error_response(&self.status, 0x20, "seekl takes no parameters");
        }

        debug!(target: "cdrom", "cdrom seekl");

        self.status.set_seeking(true);
        let seeking_status = self.status.set_seeking(false);

        CommandResponse::new()
            .int3([seeking_status], AVG_1ST_RESP_GENERIC)
            .int2([self.status.0], AVG_1ST_RESP_GENERIC + AVG_2ND_RESP_SEEKL)
    }

    pub fn seekp(&mut self) -> CommandResponse {
        if !self.parameters.is_empty() {
            return error_response(&self.status, 0x20, "seekp takes no parameters");
        }

        debug!(target: "cdrom", "cdrom seekp");

        self.status.set_seeking(true);
        let seeking_status = self.status.set_seeking(false);

        CommandResponse::new()
            .int3([seeking_status], AVG_1ST_RESP_GENERIC)
            .int2([self.status.0], AVG_1ST_RESP_GENERIC + AVG_2ND_RESP_SEEKL)
    }

    pub fn setmode(&mut self) -> CommandResponse {
        if self.parameters.len() != 1 {
            return error_response(&self.status, 0x20, "setmode expects 1 parameter");
        }

        debug!(target: "cdrom", params=?self.parameters, "cdrom set mode");

        self.mode.set_value(self.parameters[0]);

        CommandResponse::new().int3([self.status.0], AVG_1ST_RESP_GENERIC)
    }

    pub fn reads(&mut self) -> CommandResponse {
        if !self.parameters.is_empty() {
            return error_response(&self.status, 0x20, "reads takes no parameters");
        }

        debug!(target: "cdrom", "cdrom reads");

        self.readn()
    }

    pub fn readn(&mut self) -> CommandResponse {
        if !self.parameters.is_empty() {
            return error_response(&self.status, 0x20, "readn takes no parameters");
        }

        debug!(target: "cdrom", "cdrom readn");

        self.status.set_reading(true);

        CommandResponse::new()
            .int3([self.status.0], AVG_1ST_RESP_GENERIC)
            .int1(AVG_1ST_RESP_GENERIC + self.mode.speed.transform(AVG_RATE_INT1))
    }

    pub fn pause(&mut self) -> CommandResponse {
        if !self.parameters.is_empty() {
            return error_response(&self.status, 0x20, "pause takes no parameters");
        }

        debug!(target: "cdrom", "cdrom pause");

        let before = self.status.0;
        self.status.set_reading(false);
        self.status.set_playing(false);

        CommandResponse::new()
            .int3([before], AVG_1ST_RESP_GENERIC)
            .int2(
                [self.status.0],
                AVG_1ST_RESP_GENERIC + self.mode.speed.transform(AVG_2ND_RESP_PAUSE),
            )
    }

    pub fn init(&mut self) -> CommandResponse {
        if !self.parameters.is_empty() {
            return error_response(&self.status, 0x20, "init takes no parameters");
        }

        debug!(target: "cdrom", "cdrom init");

        self.mode.set_value(0x20);

        if let Some(disk) = self.disk.as_mut() {
            disk.reset_read_head();
        }

        let before = self.status.enable_motor();

        CommandResponse::new()
            .int3([before], AVG_1ST_RESP_INIT)
            .int2([self.status.0], AVG_1ST_RESP_GENERIC + AVG_2ND_RESP_SEEKL)
    }

    pub fn set_filter(&mut self) -> CommandResponse {
        if self.parameters.len() != 2 {
            return error_response(&self.status, 0x20, "set_filter expects 2 parameters");
        }

        debug!(target: "cdrom", "cdrom set filter");

        self.filter_file = self.parameters[0];
        self.filter_channel = self.parameters[1];

        CommandResponse::new().int3([self.status.0], AVG_1ST_RESP_INIT)
    }

    pub fn play(&mut self) -> CommandResponse {
        debug!(target: "cdrom", params=?self.parameters, "cdrom play");

        let _track = self.parameters.first(); // Optional

        self.status.set_playing(true);

        CommandResponse::new()
            .int3([self.status.0], AVG_1ST_RESP_INIT)
            .int1(AVG_RATE_INT1)
    }

    // stubbed audio command
    pub fn demute(&mut self) -> CommandResponse {
        if !self.parameters.is_empty() {
            return error_response(&self.status, 0x20, "demute takes no parameters");
        }

        debug!(target: "cdrom", "cdrom demute");

        self.audio_muted = false;

        CommandResponse::new().int3([self.status.0], AVG_1ST_RESP_INIT)
    }

    pub fn get_tn(&self) -> CommandResponse {
        if !self.parameters.is_empty() {
            return error_response(&self.status, 0x20, "get_tn takes no parameters");
        }

        debug!(target: "cdrom", "cdrom get tn");

        let disk = &self.disk.as_ref().expect("get_tn inserted disk");

        let first_track = to_bcd(disk.first_track_id()).expect("first track id is valid bcd");
        let last_track = to_bcd(disk.last_track_id()).expect("last track id is valid bcd");

        CommandResponse::new().int3([self.status.0, first_track, last_track], AVG_1ST_RESP_INIT)
    }

    pub fn get_td(&self) -> CommandResponse {
        if self.parameters.len() != 1 {
            return error_response(&self.status, 0x20, "get_td expects 1 parameter");
        }

        debug!(target: "cdrom", "cdrom get td");

        let disk = &self.disk.as_ref().expect("get_td inserted disk");
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
            [
                self.status.0,
                to_bcd(mm).expect("valid bcd minutes"),
                to_bcd(ss).expect("valid bcd seconds"),
            ],
            AVG_1ST_RESP_INIT,
        )
    }

    pub fn stop(&mut self) -> CommandResponse {
        if !self.parameters.is_empty() {
            return error_response(&self.status, 0x20, "stop takes no parameters");
        }

        debug!(target: "cdrom", "cdrom stop");

        self.status.set_reading(false);
        let after_reading = self.status.0;

        if let Some(cd) = self.disk.as_mut() {
            cd.seek_location(0, 2, 0);
        }

        self.status.disable_motor();

        let delay = match self.mode.speed {
            Speed::Normal => 0x0D3_8ACA,
            Speed::Double => 0x18A_6076,
        };

        CommandResponse::new()
            .int3([after_reading], AVG_1ST_RESP_INIT)
            .int2([self.status.0], delay)
    }

    pub fn get_locp(&self) -> CommandResponse {
        if !self.parameters.is_empty() {
            return error_response(&self.status, 0x20, "get_locp takes no parameters");
        }

        debug!(target: "cdrom", "cdrom getlocp");

        let disk = self.disk.as_ref().expect("get_locp inserted disk");
        let info = disk
            .current_position_info()
            .map(|x| to_bcd(x).expect("track position is valid bcd"));

        CommandResponse::new().int3(info, AVG_1ST_RESP_GENERIC)
    }

    pub fn get_locl(&self) -> CommandResponse {
        if !self.parameters.is_empty() {
            return error_response(&self.status, 0x20, "get_locl takes no parameters");
        }

        if self.status.seeking() {
            return error_response(&self.status, 0x80, "get_locl while seeking");
        }

        if self.mode.cdda_enabled {
            return error_response(&self.status, 0x80, "get_locl on audio sector");
        }

        debug!(target: "cdrom", "cdrom getlocl");

        let disk = self.disk.as_ref().expect("get_locl inserted disk");
        let info = disk.current_header_info();

        CommandResponse::new().int3(info, AVG_1ST_RESP_GENERIC)
    }
}

fn error_response(stat: &Status, err_byte: u8, err: &str) -> CommandResponse {
    error!(err, "CDROM error");
    CommandResponse::new().int5([stat.with_error(), err_byte], AVG_1ST_RESP_INIT)
}

const fn from_bcd(bcd: u8) -> Option<u8> {
    let tens = bcd >> 4;
    let ones = bcd & 0x0F;

    if tens <= 9 && ones <= 9 {
        Some(tens * 10 + ones)
    } else {
        None
    }
}

const fn to_bcd(val: u8) -> Option<u8> {
    if val > 99 {
        return None;
    }

    let tens = val / 10;
    let ones = val % 10;

    Some((tens << 4) | ones)
}

fn validate_seek(mins: u8, secs: u8, sect: u8) -> Option<(u8, u8, u8)> {
    let mins = from_bcd(mins)?;
    let secs = from_bcd(secs)?;
    let sect = from_bcd(sect)?;

    if secs > 59 || sect > 74 {
        return None;
    }

    Some((mins, secs, sect))
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum ResponseType {
    INT3(ArrayVec<u8, 8>),
    INT2(ArrayVec<u8, 8>),
    INT5([u8; 2]),
    INT1,
}

impl From<&ResponseType> for u8 {
    fn from(rt: &ResponseType) -> Self {
        match rt {
            ResponseType::INT1 => 1,
            ResponseType::INT2(_) => 2,
            ResponseType::INT3(_) => 3,
            ResponseType::INT5(_) => 5,
        }
    }
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
