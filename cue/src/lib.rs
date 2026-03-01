mod builder;
mod parser;
mod scanner;

pub use builder::CdDisk;

use std::path::{Path, PathBuf};

use anyhow::{Ok, anyhow};

use builder::CueBuilder;
use parser::CueParser;
use scanner::Scanner;

pub fn build_disk<P: AsRef<Path>>(cue_path: P) -> anyhow::Result<CdDisk> {
    let cue_file = std::fs::read(cue_path.as_ref())?;
    let tokens = Scanner::with_source(cue_file).tokenize()?;
    let cue_sheet = CueParser::new(tokens).parse_cuesheet()?;

    let parent_dir = cue_path.as_ref().parent().unwrap();
    CueBuilder::new(parent_dir).build_disk(cue_sheet)
}

#[derive(Debug)]
struct CueSheet {
    files: Vec<File>,
}

#[derive(Debug)]
struct File {
    #[expect(unused)]
    file_type: FileType,

    path: PathBuf,
    tracks: Vec<Track>,
}

#[derive(Debug)]
enum FileType {
    Binary,
}

#[derive(Debug)]
pub struct Track {
    pub id: u8,
    pub indexes: Vec<TrackIndex>,
    pub track_type: TrackType,
}

impl Track {
    pub fn single() -> Self {
        Self {
            track_type: TrackType::Mode2_2352,
            id: 1,
            indexes: vec![TrackIndex { id: 1, lba: 0 }],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrackType {
    Audio,
    Mode2_2352,
}

#[derive(Debug)]
pub struct TrackIndex {
    pub id: u8,
    pub lba: usize,
}
