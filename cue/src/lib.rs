mod builder;
mod parser;
mod scanner;

use std::path::{Path, PathBuf};

use anyhow::{Ok, anyhow};
use scanner::Scanner;

use crate::{builder::CueBuilder, parser::CueParser, scanner::CdTime};

pub fn build_disk<P: AsRef<Path>>(cue_path: P) -> anyhow::Result<Vec<u8>> {
    let cue_file = std::fs::read(cue_path.as_ref())?;
    let tokens = Scanner::with_source(cue_file).tokenize()?;
    let cue_sheet = CueParser::new(tokens).parse_cuesheet()?;

    let parent_dir = cue_path.as_ref().parent().unwrap();
    CueBuilder::new(cue_sheet, parent_dir).build_binary()
}

#[derive(Debug)]
struct CueSheet {
    files: Vec<File>,
}

#[derive(Debug)]
struct File {
    path: PathBuf,
    file_type: FileType,
    tracks: Vec<Track>,
}

#[derive(Debug)]
enum FileType {
    Binary,
}

#[derive(Debug)]
struct Track {
    id: u32,
    track_type: TrackType,
    indexes: Vec<TrackIndex>,
}

#[derive(Debug)]
enum TrackType {
    Audio,
    Mode2_2352,
}

#[derive(Debug)]
struct TrackIndex {
    id: u32,
    timestamp: CdTime,
}
