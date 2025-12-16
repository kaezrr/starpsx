use std::{error::Error, path::Path};

pub struct CdImage {
    _data: Box<[u8]>,
}

impl CdImage {
    pub fn from_path(path: &Path) -> Result<Self, Box<dyn Error>> {
        let data = std::fs::read(path)?;

        Ok(Self {
            _data: data.into_boxed_slice(),
        })
    }
}
