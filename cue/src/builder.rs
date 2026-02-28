use super::*;

const SECTOR_SIZE: usize = 0x930;

pub struct CueBuilder<'a> {
    parent_dir: &'a Path,
}

impl<'a> CueBuilder<'a> {
    pub fn new(parent_dir: &'a Path) -> Self {
        Self { parent_dir }
    }

    pub fn build_binary(self, cue_sheet: CueSheet) -> anyhow::Result<CdDisk> {
        let mut sectors = Vec::new();
        let total_tracks = cue_sheet
            .files
            .iter()
            .fold(0, |acc, elem| acc + elem.tracks.len());

        let mut tracks = Vec::with_capacity(total_tracks);

        for file in cue_sheet.files {
            let file_path = self.parent_dir.join(&file.path);
            let file_data = std::fs::read(&file_path)?;

            tracks.extend(file.tracks);

            sectors.extend(vec![0u8; SECTOR_SIZE * 75 * 2]);
            sectors.extend(file_data);
        }

        Ok(CdDisk { tracks, sectors })
    }
}

pub struct CdDisk {
    pub sectors: Vec<u8>,
    pub tracks: Vec<Track>,
}
