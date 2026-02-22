mod scanner;

use scanner::{Scanner, Token};

pub fn parse_cue(input: Vec<u8>) -> anyhow::Result<()> {
    let tokens = Scanner::with_source(input).scan_tokens()?;

    for token in tokens {
        println!("{token:?}");
    }

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

    pub fn parse(mut self) -> anyhow::Result<CueSheet> {
        todo!()
    }
}

#[derive(Debug)]
struct CueSheet;

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
