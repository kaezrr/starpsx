use anyhow::{Ok, anyhow};

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

    pub fn scan_tokens(mut self) -> anyhow::Result<Vec<Token>> {
        while !self.is_at_end() {
            let token = self.scan_token()?;
            self.tokens.push(token);
        }

        Ok(self.tokens)
    }

    fn scan_token(&mut self) -> anyhow::Result<Token> {
        // Skip meaningless whitespace
        while !self.is_at_end() && matches!(self.peek(), ' ' | '\t' | '\r') {
            self.advance();
        }

        if self.is_at_end() {
            return Ok(Token::Eof);
        }

        self.start = self.current;

        match self.advance() {
            '\n' => Ok(Token::Newline),

            '"' => Ok(Token::String(self.string()?)),

            c if c.is_ascii_alphabetic() => self.keyword(),

            c if c.is_ascii_digit() => self.number_or_time(),

            c => Err(anyhow!("Unexpected character '{}'", c)),
        }
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
        Ok(Token::CdTime(word.to_string()))
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

#[derive(Debug, PartialEq)]
pub enum Token {
    File,
    Track,
    Index,
    Pregap,
    Postgap,

    // File types
    Binary,
    Motorola,
    Aiff,
    Wave,
    Mp3,

    // Track types
    Audio,
    Cdg,
    Mode1_2048,
    Mode1_2352,
    Mode2_2336,
    Mode2_2352,
    Cdi2336,
    Cdi2352,

    String(String),
    Number(u32),
    CdTime(String),

    // Misc whatever
    Performer,
    Songwriter,
    Title,

    Newline,
    Eof,
}
