use super::CueSheet;
use super::File;
use super::FileType;
use super::PathBuf;
use super::Track;
use super::TrackIndex;
use super::TrackType;
use crate::scanner::Token;

pub struct CueParser {
    tokens: Vec<Token>,
    current: usize,
}

impl CueParser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    /// cue -> file*
    pub fn parse_cuesheet(mut self) -> anyhow::Result<CueSheet> {
        let mut files = Vec::new();

        while !self.is_at_end() && self.peek() != &Token::Eof {
            files.push(self.parse_file()?);
        }

        Ok(CueSheet { files })
    }

    /// file -> "FILE" filename filetype "\n" track*
    fn parse_file(&mut self) -> anyhow::Result<File> {
        let Token::File = self.advance() else {
            anyhow::bail!("Expect 'FILE'.");
        };

        let Token::String(name) = self.advance().clone() else {
            anyhow::bail!("Expect file path.");
        };

        let file_type = match self.advance() {
            Token::Binary => FileType::Binary,
            t => anyhow::bail!("File type {t:?} not implemented."),
        };

        let Token::Newline = self.advance() else {
            anyhow::bail!("Expect newline after file.");
        };

        let mut tracks = Vec::new();

        while let Token::Track = self.peek() {
            tracks.push(self.parse_track()?);
        }

        Ok(File {
            path: PathBuf::from(name),
            format: file_type,
            tracks,
        })
    }

    /// track -> "TRACK" tracknumber tracktype "\n" flags? index*
    fn parse_track(&mut self) -> anyhow::Result<Track> {
        let Token::Track = self.advance() else {
            anyhow::bail!("Expect 'TRACK'.");
        };

        let &Token::Number(id) = self.advance() else {
            anyhow::bail!("Expect track id.");
        };

        let track_type = match self.advance() {
            Token::Audio => TrackType::Audio,
            Token::Mode2_2352 => TrackType::Mode2_2352,
            t => anyhow::bail!("Track type {t:?} not implemented."),
        };

        let Token::Newline = self.advance() else {
            anyhow::bail!("Expect newline after track.");
        };

        let mut indexes = Vec::new();

        // Consume useless flags
        if let Token::Flags = self.peek() {
            self.parse_flags()?;
        }

        while let Token::Index = self.peek() {
            indexes.push(self.parse_index()?);
        }

        Ok(Track {
            id,
            indexes,
            track_type,
        })
    }

    /// flags -> "FLAGS" "DCP" "\n"
    fn parse_flags(&mut self) -> anyhow::Result<()> {
        let Token::Flags = self.advance() else {
            anyhow::bail!("Expect 'FLAGS'.");
        };

        let Token::Dcp = self.advance() else {
            anyhow::bail!("Expect 'DCP'");
        };

        let Token::Newline = self.advance() else {
            anyhow::bail!("Expect newline after flags.");
        };

        Ok(())
    }

    /// index -> "INDEX" indexnumber sector "\n"
    fn parse_index(&mut self) -> anyhow::Result<TrackIndex> {
        let Token::Index = self.advance() else {
            anyhow::bail!("Expect 'INDEX'.");
        };

        let &Token::Number(id) = self.advance() else {
            anyhow::bail!("Expect index number.");
        };

        let &Token::CdTime(lba) = self.advance() else {
            anyhow::bail!("Expect track time.");
        };

        let Token::Newline = self.advance() else {
            anyhow::bail!("Expect newline after index.");
        };

        Ok(TrackIndex { id, lba })
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len()
    }

    fn advance(&mut self) -> &Token {
        self.current += 1;
        &self.tokens[self.current - 1]
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }
}
