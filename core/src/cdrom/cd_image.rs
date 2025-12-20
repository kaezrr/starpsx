use std::{error::Error, path::Path};

use crate::consts::SECTOR_SIZE;

pub struct CdImage {
    _data: Box<[u8]>,
    _read_head: usize,
}

impl CdImage {
    pub fn from_path(path: &Path) -> Result<Self, Box<dyn Error>> {
        let data = std::fs::read(path)?;

        Ok(Self {
            _data: data.into_boxed_slice(),
            _read_head: 0,
        })
    }

    pub fn seek_location(&mut self, mins: u8, secs: u8, sect: u8) {
        let total_sectors = ((mins as usize) * 60 * 75) + ((secs as usize) * 75) + (sect as usize);
        self._read_head = total_sectors * SECTOR_SIZE;
    }
}
