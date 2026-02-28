use tracing::debug;

use super::SectorSize;
use crate::consts::SECTOR_SIZE;
use std::collections::VecDeque;

pub struct CdImage {
    read_head: usize,
    data: Box<[u8]>,
    tracks: Box<[cue::Track]>,
}

impl CdImage {
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        // Add 2 seconds of zero padding to disk image (missing in rips)
        let mut data = vec![0u8; 2 * 75 * SECTOR_SIZE];
        data.extend(bytes);

        Self {
            read_head: 0,
            data: data.into_boxed_slice(),
            tracks: Box::new([cue::Track::single()]),
        }
    }

    pub fn from_disk(disk: cue::CdDisk) -> Self {
        Self {
            read_head: 0,
            data: disk.sectors,
            tracks: disk.tracks,
        }
    }

    pub fn first_track_id(&self) -> u8 {
        self.tracks.first().unwrap().id
    }

    pub fn last_track_id(&self) -> u8 {
        self.tracks.last().unwrap().id
    }

    pub fn track_mm_ss_ff(&self, track_id: u8) -> (u8, u8, u8) {
        let track = &self.tracks[track_id as usize - 1];
        mm_ss_ff(track.indexes[0].lba)
    }

    pub fn last_track_end(&self) -> (u8, u8, u8) {
        let index = self.data.len();
        mm_ss_ff(index)
    }

    pub fn seek_location(&mut self, mins: u8, secs: u8, sect: u8) {
        let total_sectors = ((mins as usize) * 60 * 75) + ((secs as usize) * 75) + (sect as usize);
        self.read_head = total_sectors * SECTOR_SIZE;
    }

    pub fn read_sector_and_advance(&mut self, sect_size: SectorSize) -> VecDeque<u8> {
        debug!(
            target: "cdrom",
            LBA = self.read_head / SECTOR_SIZE,
            read_head = %mm_ss_ff_str(self.read_head),
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

        VecDeque::from_iter(sector_read.iter().copied())
    }
}

fn mm_ss_ff(read_head: usize) -> (u8, u8, u8) {
    let sectors = read_head / SECTOR_SIZE;
    let secs = sectors / 75;
    let sect = sectors % 75;
    let mins = secs / 60;
    let secs = secs % 60;

    (mins as u8, secs as u8, sect as u8)
}

fn mm_ss_ff_str(read_head: usize) -> String {
    let i = mm_ss_ff(read_head);
    format!("{}:{}:{}", i.0, i.1, i.2)
}
