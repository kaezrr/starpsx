mod scanner;

use scanner::{Scanner, Token};

pub fn parse_cue(input: Vec<u8>) -> anyhow::Result<()> {
    let tokens = Scanner::with_source(input).scan_tokens()?;
    let cuesheet = Parser::new(tokens).parse_cuesheet()?;

    eprintln!("{cuesheet:?}");

    Ok(())
}

struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    /// cue -> file*
    pub fn parse_cuesheet(mut self) -> anyhow::Result<CueSheet> {
        todo!()
    }

    /// file -> "FILE" filename filetype track+
    fn parse_file(&mut self) -> anyhow::Result<Track> {
        todo!()
    }

    /// track -> "TRACK" tracknumber tracktype pregap? index+
    fn parse_track(&mut self) -> anyhow::Result<Track> {
        todo!()
    }

    /// index -> "INDEX" indexnumber sector
    fn parse_index(&mut self) -> anyhow::Result<Index> {
        todo!()
    }
}

#[derive(Debug)]
struct CueSheet;

#[derive(Debug)]
struct File;

#[derive(Debug)]
struct Track;

#[derive(Debug)]
struct Index;

#[test]
fn parse_cue_file() {
    use std::path::PathBuf;
    use std::str::FromStr;

    const GAME_DIR: &str = "/home/kaezr/Projects/starpsx/stuff/games/";

    let mut game_dir = PathBuf::from_str(GAME_DIR).expect("game path");
    game_dir.push("ridge/Ridge Racer (USA).cue");

    let content = std::fs::read(game_dir.as_path()).expect("file");

    parse_cue(content).expect("Parser should succeed");
}
