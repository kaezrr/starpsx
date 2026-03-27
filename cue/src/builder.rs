use std::fmt::Debug;

use super::CueSheet;
use super::File;
use super::Path;
use super::Track;
use crate::TrackType;

pub const SECTOR_SIZE: usize = 0x930;
const SEC_2: usize = SECTOR_SIZE * 75 * 2;

pub struct CueBuilder<'a> {
    parent_dir: &'a Path,
    current: usize,
    sectors: Vec<u8>,
    tracks: Vec<Track>,
}

impl<'a> CueBuilder<'a> {
    pub const fn new(parent_dir: &'a Path) -> Self {
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
            for index in &mut track.indexes {
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

impl Debug for CdDisk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn lba_to_msf(lba: usize) -> String {
            let frames = lba % 75;
            let seconds = (lba / 75) % 60;
            let minutes = lba / 75 / 60;
            format!("{minutes:02}:{seconds:02}:{frames:02}")
        }

        fn track_start(track: &Track) -> usize {
            track
                .indexes
                .iter()
                .find(|idx| idx.id == 1)
                .or_else(|| track.indexes.first())
                .map_or(0, |idx| idx.lba)
        }

        let total_sectors = self.sectors.len() / SECTOR_SIZE;

        writeln!(f, "{:<9} {:<12} {:<10} Length", "#", "Mode", "Start")?;
        writeln!(f, "{}", "-".repeat(45))?;

        for (i, track) in self.tracks.iter().enumerate() {
            let mode = match track.track_type {
                TrackType::Audio => "Audio",
                TrackType::Mode2_2352 => "Mode2/2352",
            };

            let start_lba = track_start(track) / SECTOR_SIZE;
            let end_lba = self
                .tracks
                .get(i + 1)
                .map_or(total_sectors, |t| track_start(t) / SECTOR_SIZE);

            writeln!(
                f,
                "{:<9} {:<12} {:<10} {}",
                format!("Track {}", track.id),
                mode,
                lba_to_msf(start_lba),
                lba_to_msf(end_lba.saturating_sub(start_lba)),
            )?;
        }

        Ok(())
    }
}
