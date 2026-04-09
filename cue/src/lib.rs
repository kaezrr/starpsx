mod builder;
mod parser;
mod scanner;

use std::path::Path;
use std::path::PathBuf;

use builder::CueBuilder;
pub use builder::Disc;
use parser::CueParser;
use scanner::Scanner;

/// # Errors
///
/// Returns an error if:
/// * The CUE file or its referenced binary tracks cannot be read.
/// * The file content contains invalid tokens or malformed syntax.
/// * The disk layout is invalid or references missing resources.
pub fn build_disk<P: AsRef<Path>>(cue_path: P) -> anyhow::Result<Disc> {
    let cue_file = std::fs::read(cue_path.as_ref())?;
    let tokens = Scanner::with_source(cue_file).tokenize()?;
    let cue_sheet = CueParser::new(tokens).parse_cuesheet()?;

    let parent_dir = cue_path.as_ref().parent().unwrap_or_else(|| Path::new("."));
    CueBuilder::new(parent_dir).build_disk(cue_sheet)
}

#[derive(Debug)]
struct CueSheet {
    files: Vec<File>,
}

#[derive(Debug)]
struct File {
    #[expect(unused)]
    format: FileType,
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
    #[must_use]
    pub fn single() -> Self {
        Self {
            track_type: TrackType::Mode2_2352,
            id: 1,
            indexes: vec![TrackIndex { id: 1, lba: 0 }],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackType {
    Audio,
    Mode2_2352,
}

#[derive(Debug)]
pub struct TrackIndex {
    pub id: u8,
    pub lba: usize,
}
