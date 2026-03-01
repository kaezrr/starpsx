use super::*;

pub(crate) const SECTOR_SIZE: usize = 0x930;
const SEC_2: usize = SECTOR_SIZE * 75 * 2;

pub struct CueBuilder<'a> {
    parent_dir: &'a Path,
    current: usize,
    sectors: Vec<u8>,
    tracks: Vec<Track>,
}

impl<'a> CueBuilder<'a> {
    pub fn new(parent_dir: &'a Path) -> Self {
        Self {
            parent_dir,
            current: 0,
            sectors: Vec::new(),
            tracks: Vec::new(),
        }
    }

    pub fn build_disk(mut self, cue_sheet: CueSheet) -> anyhow::Result<CdDisk> {
        self.current += SEC_2;
        self.sectors.extend_from_slice(&vec![0; SEC_2]);

        for file in cue_sheet.files {
            self.insert_file(file)?;
        }

        Ok(CdDisk {
            sectors: self.sectors.into_boxed_slice(),
            tracks: self.tracks.into_boxed_slice(),
        })
    }

    fn insert_file(&mut self, mut file: File) -> anyhow::Result<()> {
        let mut data = std::fs::read(self.parent_dir.join(&file.path))?;

        let first_index = &mut file.tracks[0].indexes[0];
        if first_index.id == 1 && first_index.lba == SEC_2 {
            data = data.split_off(SEC_2);
            first_index.lba = 0;
        }

        for mut track in file.tracks {
            for index in track.indexes.iter_mut() {
                index.lba += self.current;
            }
            self.tracks.push(track);
        }

        self.current += data.len();
        self.sectors.extend(data);
        Ok(())
    }
}

pub struct CdDisk {
    pub sectors: Box<[u8]>,
    pub tracks: Box<[Track]>,
}
