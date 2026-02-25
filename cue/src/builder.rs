use super::*;

const BYTES_PER_SECTOR: u32 = 0x930;
const TWO_SECOND_SECTORS: u32 = 75 * 2;

pub struct CueBuilder {
    cue_sheet: CueSheet,
    binary: Vec<u8>,
}

impl CueBuilder {
    pub fn new(cue_sheet: CueSheet) -> Self {
        Self {
            cue_sheet,
            binary: Vec::new(),
        }
    }

    pub fn build_binary(mut self) -> anyhow::Result<Vec<u8>> {
        for file in self.cue_sheet.files {
            self.binary.extend(std::fs::read(file.path)?);
        }

        Ok(self.binary)
    }

    pub fn push_empty_sectors(&mut self, sectors: u32) {}
}
