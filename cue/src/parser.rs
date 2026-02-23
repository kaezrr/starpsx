use crate::scanner::Token;

use super::*;

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
    fn parse_file(&mut self) -> anyhow::Result<CueFile> {
        let Token::File = self.advance() else {
            return Err(anyhow!("Expect 'FILE'."));
        };

        let Token::String(name) = self.advance().clone() else {
            return Err(anyhow!("Expect file path."));
        };

        let file_type = match self.advance() {
            Token::Binary => CueFileType::Binary,
            t => return Err(anyhow!("File type {t:?} not implemented.")),
        };

        let Token::Newline = self.advance() else {
            return Err(anyhow!("Expect newline after file."));
        };

        let mut tracks = Vec::new();

        while let Token::Track = self.peek() {
            tracks.push(self.parse_track()?);
        }

        Ok(CueFile {
            path: PathBuf::from(name),
            file_type,
            tracks,
        })
    }

    /// track -> "TRACK" tracknumber tracktype "\n" index*
    fn parse_track(&mut self) -> anyhow::Result<Track> {
        let Token::Track = self.advance() else {
            return Err(anyhow!("Expect 'TRACK'."));
        };

        let &Token::Number(id) = self.advance() else {
            return Err(anyhow!("Expect track id."));
        };

        let track_type = match self.advance() {
            Token::Audio => TrackType::Audio,
            Token::Mode2_2352 => TrackType::Mode2_2352,
            t => return Err(anyhow!("Track type {t:?} not implemented.")),
        };

        let Token::Newline = self.advance() else {
            return Err(anyhow!("Expect newline after track."));
        };

        let mut indexes = Vec::new();

        while let Token::Index = self.peek() {
            indexes.push(self.parse_index()?);
        }

        Ok(Track {
            id,
            track_type,
            indexes,
        })
    }

    /// index -> "INDEX" indexnumber sector "\n"
    fn parse_index(&mut self) -> anyhow::Result<TrackIndex> {
        let Token::Index = self.advance() else {
            return Err(anyhow!("Expect 'INDEX'."));
        };

        let &Token::Number(id) = self.advance() else {
            return Err(anyhow!("Expect index number."));
        };

        let &Token::CdTime(timestamp) = self.advance() else {
            return Err(anyhow!("Expect track time."));
        };

        let Token::Newline = self.advance() else {
            return Err(anyhow!("Expect newline after index."));
        };

        Ok(TrackIndex { id, timestamp })
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
