use super::parse::ViewError;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Tok<'a> {
    Word(&'a str),
    Str(String),
    Bound(&'a str),
    Action(&'a str),
    Equals,
    Open,
    Close,
    Newline,
    End,
}

#[derive(Clone, Debug)]
pub(super) struct Token<'a> {
    pub(super) tok: Tok<'a>,
    pub(super) line: u32,
    pub(super) col: u32,
}

struct Lexer<'a> {
    rest: &'a str,
    line: u32,
    col: u32,
}

const fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

impl<'a> Lexer<'a> {
    fn fail(&self, message: impl Into<String>) -> ViewError {
        ViewError::at(self.line, self.col, message)
    }

    fn bump(&mut self, c: char) {
        self.rest = &self.rest[c.len_utf8()..];
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
    }

    fn advance_cols(&mut self, text: &str) {
        let width = u32::try_from(text.chars().count()).unwrap_or(u32::MAX);
        self.col = self.col.saturating_add(width);
    }

    fn take_word(&mut self) -> &'a str {
        let end = self.rest.find(|c| !is_word(c)).unwrap_or(self.rest.len());
        let (word, rest) = self.rest.split_at(end);
        self.rest = rest;
        self.advance_cols(word);
        word
    }

    fn sigil_word(&mut self, mark: char) -> Result<&'a str, ViewError> {
        self.bump(mark);
        let word = self.take_word();
        if word.is_empty() {
            return Err(self.fail(format!("`{mark}` needs a name after it")));
        }
        Ok(word)
    }

    fn escape(&mut self) -> Result<char, ViewError> {
        self.bump('\\');
        let Some(c) = self.rest.chars().next() else {
            return Err(self.fail("the text has no closing `\"`"));
        };
        let out = match c {
            'n' => '\n',
            '"' | '\\' => c,
            _ => return Err(self.fail(format!("`\\{c}` is not an escape: use \\n, \\\", or \\\\"))),
        };
        self.bump(c);
        Ok(out)
    }

    fn string(&mut self) -> Result<String, ViewError> {
        self.bump('"');
        let mut out = String::new();
        loop {
            match self.rest.chars().next() {
                None | Some('\n') => return Err(self.fail("the text has no closing `\"`")),
                Some('"') => {
                    self.bump('"');
                    return Ok(out);
                }
                Some('\\') => out.push(self.escape()?),
                Some(c) => {
                    self.bump(c);
                    out.push(c);
                }
            }
        }
    }

    fn skip_comment(&mut self) {
        let end = self.rest.find('\n').unwrap_or(self.rest.len());
        let (comment, rest) = self.rest.split_at(end);
        self.advance_cols(comment);
        self.rest = rest;
    }

    fn one(&mut self, c: char, tok: Tok<'a>) -> Tok<'a> {
        self.bump(c);
        tok
    }

    fn next(&mut self, c: char) -> Result<Option<Tok<'a>>, ViewError> {
        Ok(Some(match c {
            ' ' | '\t' | '\r' => {
                self.bump(c);
                return Ok(None);
            }
            '/' if self.rest.starts_with("//") => {
                self.skip_comment();
                return Ok(None);
            }
            '\n' => self.one(c, Tok::Newline),
            '=' => self.one(c, Tok::Equals),
            '{' => self.one(c, Tok::Open),
            '}' => self.one(c, Tok::Close),
            '"' => Tok::Str(self.string()?),
            '$' => Tok::Bound(self.sigil_word(c)?),
            '@' => Tok::Action(self.sigil_word(c)?),
            _ if is_word(c) => Tok::Word(self.take_word()),
            _ => return Err(self.fail(format!("`{c}` is not part of a view file"))),
        }))
    }
}

pub(super) fn lex(source: &str) -> Result<Vec<Token<'_>>, ViewError> {
    let mut lexer = Lexer {
        rest: source,
        line: 1,
        col: 1,
    };
    let mut tokens = Vec::new();
    while let Some(c) = lexer.rest.chars().next() {
        let (line, col) = (lexer.line, lexer.col);
        if let Some(tok) = lexer.next(c)? {
            tokens.push(Token { tok, line, col });
        }
    }
    tokens.push(Token {
        tok: Tok::End,
        line: lexer.line,
        col: lexer.col,
    });
    Ok(tokens)
}
