use crate::consts::{AVG_DELAY_FIRST_RES, AVG_DELAY_GET_ID, CDROM_VERSION, GET_ID_RESPONSE};
use tracing::debug;

use super::*;

#[derive(PartialEq, Clone)]
pub enum ResponseType {
    INT3(Vec<u8>),
    INT2(Vec<u8>),
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
                    AVG_DELAY_FIRST_RES,
                );
            }
            _ => unimplemented!("cdrom command Test {cmd:02x}"),
        };
        responses
    }

    pub fn nop(&mut self) -> CommandResponse {
        debug!(status=?self.status.0, "cdrom nop command");

        let mut responses = CommandResponse::default();
        responses.push(ResponseType::INT3(vec![self.status.0]), AVG_DELAY_FIRST_RES);
        responses
    }

    pub fn get_id(&mut self) -> CommandResponse {
        debug!("cdrom get id");

        let mut responses = CommandResponse::default();
        responses.push(ResponseType::INT3(vec![self.status.0]), AVG_DELAY_FIRST_RES);
        responses.push(
            ResponseType::INT2(GET_ID_RESPONSE.into()),
            AVG_DELAY_FIRST_RES + AVG_DELAY_GET_ID,
        );
        responses
    }

    pub fn set_loc(&mut self) -> CommandResponse {
        debug!(?self.parameters, "cdrom set loc");
        todo!("cdrom loc")
    }
}
