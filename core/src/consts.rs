pub const TARGET_FPS: u64 = 60;
pub const LINE_DURATION: u64 = 2172;
pub const HBLANK_DURATION: u64 = 390;

pub const BAUDRATE_TRANSFER_DELAY: u64 = 1088;

pub const AVG_DELAY_FIRST_RES: u64 = 0xC4E1;
pub const AVG_DELAY_GET_ID: u64 = 0x4A00;

pub const CDROM_VERSION: [u8; 4] = [0x95, 0x05, 0x16, 0xC1];
pub const GET_ID_RESPONSE: [u8; 8] = [0x02, 0x00, 0x20, 0x00, 0x53, 0x43, 0x45, 0x4A];
