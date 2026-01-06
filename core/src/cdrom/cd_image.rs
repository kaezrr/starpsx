use tracing::debug;

use super::SectorSize;
use crate::consts::SECTOR_SIZE;
use std::{collections::VecDeque, error::Error, path::Path};

pub struct CdImage {
    data: Box<[u8]>,
    read_head: usize,
}

impl CdImage {
    pub fn from_path(path: &Path) -> Result<Self, Box<dyn Error>> {
        // Add 2 seconds of zero padding to disk image (missing in rips)
        let mut data = vec![0u8; 2 * 75 * SECTOR_SIZE];
        data.append(&mut std::fs::read(path)?);

        Ok(Self {
            data: data.into_boxed_slice(),
            read_head: 0,
        })
    }

    pub fn seek_location(&mut self, mins: u8, secs: u8, sect: u8) {
        let total_sectors = ((mins as usize) * 60 * 75) + ((secs as usize) * 75) + (sect as usize);
        self.read_head = total_sectors * SECTOR_SIZE;
    }

    pub fn read_sector_and_advance(&mut self, sect_size: SectorSize) -> VecDeque<u32> {
        debug!(
            LBA = self.read_head / SECTOR_SIZE - 150,
            read_head = %read_head_to_disk_str(self.read_head),
            ?sect_size,
            "reading sector"
        );
        debug_assert!(self.read_head + SECTOR_SIZE <= self.data.len());

        let sector = &self.data[self.read_head..self.read_head + SECTOR_SIZE];
        self.read_head += SECTOR_SIZE;

        let sector_read = match sect_size {
            SectorSize::DataOnly => &sector[0x18..0x818],
            SectorSize::WholeSectorExceptSyncBytes => &sector[0xC..],
        };

        let words = bytemuck::cast_slice::<u8, u32>(sector_read);
        let mut buffer = VecDeque::with_capacity(words.len());
        buffer.extend(words.iter().copied());
        buffer
    }
}

fn read_head_to_disk_str(read_head: usize) -> String {
    let sectors = read_head / SECTOR_SIZE;
    let secs = sectors / 75;
    let sect = sectors % 75;
    let mins = secs / 60;
    let secs = secs % 60;

    format!("{:02}:{:02}:{:02}", mins, secs, sect)
}
