mod parser;
mod scanner;

use std::path::{Path, PathBuf};

use anyhow::{Ok, anyhow};
use scanner::{Scanner, Token};

use crate::{parser::CueParser, scanner::CdTime};

pub fn build_binary(cue_path: &Path) -> anyhow::Result<()> {
    let cue_file = std::fs::read(cue_path)?;
    let tokens = Scanner::with_source(cue_file).tokenize()?;
    let cue_sheet = CueParser::new(tokens).parse_cuesheet()?;
    Ok(())
}

#[derive(Debug)]
struct CueSheet {
    files: Vec<CueFile>,
}

#[derive(Debug)]
struct CueFile {
    path: PathBuf,
    file_type: CueFileType,
    tracks: Vec<Track>,
}

#[derive(Debug)]
enum CueFileType {
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

#[test]
fn parse_cue_files() {
    use std::path::Path;

    const GAME_DIR: &str = "/home/kaezr/Projects/starpsx/stuff/games/";
    const GAMES: [&str; 8] = [
        "battle-arena-toshiden/Battle Arena Toshinden (USA).cue",
        "crash/Crash Bandicoot (USA).cue",
        "ew-jim-2/Earthworm Jim 2 (Europe).cue",
        "mortal-kombat-2/Mortal Kombat II (Japan).cue",
        "puzzle-bobble-2/Puzzle Bobble 2 (Japan).cue",
        "ridge/Ridge Racer (USA).cue",
        "silent-hill/Silent Hill (USA).cue",
        "spyro/Spyro the Dragon (USA).cue",
    ];

    let base = Path::new(GAME_DIR);
    for game in GAMES {
        let path = base.join(game);
        build_binary(&path).unwrap_or_else(|e| panic!("Parser failed for {}: {e}", path.display()));
    }
}
