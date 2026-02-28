use super::*;

pub struct Scanner {
    source: Vec<u8>,
    tokens: Vec<Token>,

    start: usize,
    current: usize,
}

impl Scanner {
    pub fn with_source(source: Vec<u8>) -> Self {
        Self {
            source,
            tokens: Vec::new(),

            start: 0,
            current: 0,
        }
    }

    pub fn tokenize(mut self) -> anyhow::Result<Vec<Token>> {
        while !self.is_at_end() {
            self.scan_token()?;
            self.start = self.current;
        }

        self.tokens.push(Token::Eof);

        Ok(self.tokens)
    }

    fn scan_token(&mut self) -> anyhow::Result<()> {
        let token = match self.advance() {
            ' ' | '\t' | '\r' => return Ok(()),

            '\n' => Token::Newline,

            '"' => Token::String(self.string()?),

            c if c.is_ascii_alphabetic() => self.keyword()?,

            c if c.is_ascii_digit() => self.number_or_time()?,

            c => return Err(anyhow!("Unexpected character '{}'", c)),
        };

        self.tokens.push(token);

        Ok(())
    }

    fn string(&mut self) -> anyhow::Result<String> {
        while !self.is_at_end() && self.peek() != '"' {
            self.advance();
        }

        if self.is_at_end() {
            return Err(anyhow!("Unterminated string"));
        }

        self.advance();

        let bytes = &self.source[self.start + 1..self.current - 1];
        let word = str::from_utf8(bytes)?;

        Ok(word.to_string())
    }

    fn keyword(&mut self) -> anyhow::Result<Token> {
        while is_alnum(self.peek()) {
            self.advance();
        }

        let bytes = &self.source[self.start..self.current];
        let keyword = str::from_utf8(bytes).expect("UTF-8");

        try_to_keyword(keyword)
    }

    fn number_or_time(&mut self) -> anyhow::Result<Token> {
        while self.peek().is_ascii_digit() {
            self.advance();
        }

        // If not followed by ':', it's a plain number
        if self.peek() != ':' {
            let bytes = &self.source[self.start..self.current];
            let word = std::str::from_utf8(bytes)?;
            return Ok(Token::Number(word.parse()?));
        }

        // We are parsing MM:SS:FF
        for _ in 0..2 {
            self.advance(); // consume ':'

            if !self.peek().is_ascii_digit() {
                return Err(anyhow!("Invalid CD time format"));
            }

            while self.peek().is_ascii_digit() {
                self.advance();
            }
        }

        let bytes = &self.source[self.start..self.current];
        let word = std::str::from_utf8(bytes)?;

        Ok(Token::CdTime(to_sectors(
            word[0..2].parse()?, // MM
            word[3..5].parse()?, // HH
            word[6..8].parse()?, // SS
        )))
    }

    fn peek(&mut self) -> char {
        self.source[self.current] as char
    }

    fn advance(&mut self) -> char {
        let ch = self.source[self.current];
        self.current += 1;
        ch as char
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }
}

fn is_alnum(ch: char) -> bool {
    ch == '/' || ch.is_ascii_alphanumeric()
}

fn try_to_keyword(s: &str) -> anyhow::Result<Token> {
    match s {
        "FILE" => Ok(Token::File),
        "BINARY" => Ok(Token::Binary),
        "TRACK" => Ok(Token::Track),
        "INDEX" => Ok(Token::Index),
        "AUDIO" => Ok(Token::Audio),
        "MODE2/2352" => Ok(Token::Mode2_2352),
        word => Err(anyhow!("Invalid keyword: -{}-", word)),
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    File,
    Track,
    Index,

    // File types
    Binary,

    // Track types
    Audio,
    Mode2_2352,

    String(String),
    Number(u8),
    CdTime(usize),

    Newline,
    Eof,
}

fn to_sectors(minutes: usize, seconds: usize, frames: usize) -> usize {
    (minutes * 60 * 75 + seconds * 75 + frames) * builder::SECTOR_SIZE
}
