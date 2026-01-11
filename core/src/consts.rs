pub const LINE_DURATION: u64 = 2172;
pub const HBLANK_DURATION: u64 = 390;

pub const BAUDRATE_TRANSFER_DELAY: u64 = 1088;

pub const SECTOR_SIZE: usize = 0x930;

pub const AVG_1ST_RESP_GENERIC: u64 = 0xC4E1;
pub const AVG_1ST_RESP_INIT: u64 = 0x13CCE;

pub const AVG_2ND_RESP_GET_ID: u64 = 0x4A00;
pub const AVG_2ND_RESP_PAUSE: u64 = 0x21181C;
pub const AVG_2ND_RESP_SEEKL: u64 = 0x6E1CD;

pub const AVG_RATE_INT1: u64 = 0x6E1CD;

pub const CDROM_VERSION: [u8; 4] = [0x95, 0x05, 0x16, 0xC1];
pub const GET_ID_RESPONSE: [u8; 8] = [0x02, 0x00, 0x20, 0x00, b'S', b'C', b'E', b'A'];
