use super::*;

const SECTOR_SIZE: usize = 0x930;

pub struct CueBuilder<'a> {
    cue_sheet: CueSheet,
    binary: Vec<u8>,
    parent_dir: &'a Path,
}

impl<'a> CueBuilder<'a> {
    pub fn new(cue_sheet: CueSheet, parent_dir: &'a Path) -> Self {
        Self {
            cue_sheet,
            binary: Vec::new(),
            parent_dir,
        }
    }

    pub fn build_binary(mut self) -> anyhow::Result<Vec<u8>> {
        for file in self.cue_sheet.files {
            self.binary.extend(vec![0; SECTOR_SIZE * 75 * 2]); // 2 seconds

            let file_path = self.parent_dir.join(file.path);
            self.binary.extend(std::fs::read(file_path)?);
        }

        Ok(self.binary)
    }
}
