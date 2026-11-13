use std::collections::HashMap;
use std::ops::Range;
use std::sync::LazyLock;

use toml::{Table, Value};
use unicode_segmentation::UnicodeSegmentation;

use crate::style::{TextFace, TextRun, Token};

static CATALOG: LazyLock<Vec<Lang>> =
    LazyLock::new(|| catalog(include_str!("highlight/languages.toml")));

/// One syntax mark and its theme token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Highlight {
    /// The first grapheme column.
    pub(crate) start: usize,
    /// The column past the last grapheme.
    pub(crate) end: usize,
    /// The token.
    pub(crate) token: Token,
}

impl Highlight {
    /// Creates a highlight of the columns `start..end` in the color of `token`.
    #[must_use]
    pub const fn new(start: usize, end: usize, token: Token) -> Self {
        Self { start, end, token }
    }

    /// The first column.
    #[must_use]
    pub const fn start(&self) -> usize {
        self.start
    }

    /// The column after the last.
    #[must_use]
    pub const fn end(&self) -> usize {
        self.end
    }

    /// The color token.
    #[must_use]
    pub const fn token(&self) -> Token {
        self.token
    }
}

#[derive(Default)]
struct Lang {
    names: Vec<String>,
    line_comments: Vec<String>,
    block_comment: Option<(String, String)>,
    nested_block_comments: bool,
    strings: Vec<char>,
    numbers: bool,
    words: HashMap<String, Token>,
    prefixes: Vec<(String, Token)>,
    markers: Vec<(String, Token)>,
    comment: Option<Token>,
    string: Option<Token>,
    number: Option<Token>,
}

/// Syntax marks per line, in the grapheme columns `code_edit` takes, for rust, sh, toml, json, lean, python, javascript, typescript, go, c/cpp, yaml, css, html, sql, markdown, and diff.
#[must_use]
pub fn highlight(language: &str, source: &str) -> Vec<Vec<Highlight>> {
    source
        .split('\n')
        .zip(byte_marks(language, source))
        .map(|(line, marks)| {
            let starts: Vec<usize> = line.grapheme_indices(true).map(|(at, _)| at).collect();
            let col = |byte: usize| {
                if line.is_char_boundary(byte) {
                    starts.partition_point(|&start| start < byte)
                } else {
                    0
                }
            };
            marks
                .into_iter()
                .map(|mark| Highlight {
                    start: col(mark.start),
                    end: col(mark.end),
                    token: mark.token,
                })
                .collect()
        })
        .collect()
}

#[must_use]
pub(crate) fn byte_marks(language: &str, source: &str) -> Vec<Vec<Highlight>> {
    let tag = language.trim().to_ascii_lowercase();
    let lang = CATALOG.iter().find(|lang| lang.names.contains(&tag));
    let mut open = 0;
    source
        .split('\n')
        .map(|line| lang.map_or_else(Vec::new, |lang| lang.line(line, &mut open)))
        .collect()
}

/// Returns the highlight language for a file path.
#[must_use]
pub(crate) fn language_for_path(path: &str) -> Option<&'static str> {
    let name = path.rsplit(['/', '\\']).next()?.to_ascii_lowercase();
    let tag = name.rsplit_once('.').map_or(name.as_str(), |(_, ext)| ext);
    CATALOG
        .iter()
        .find(|lang| lang.names.iter().any(|known| known == tag))
        .and_then(|lang| lang.names.first())
        .map(String::as_str)
}

/// Converts highlights in `range` to styled text runs.
#[must_use]
pub(crate) fn text_runs(marks: &[Highlight], range: Range<usize>) -> Vec<TextRun> {
    marks
        .iter()
        .filter_map(|mark| {
            let start = mark.start.max(range.start);
            let end = mark.end.min(range.end);
            (start < end).then(|| TextRun {
                start: start - range.start,
                end: end - range.start,
                face: TextFace {
                    color: Some(mark.token),
                    ..TextFace::default()
                },
            })
        })
        .collect()
}

fn catalog(source: &str) -> Vec<Lang> {
    let Ok(doc) = source.parse::<Table>() else {
        return Vec::new();
    };
    let styles = doc.get("styles").and_then(Value::as_table);
    let style = |name: &str| {
        styles
            .and_then(|table| table.get(name))
            .and_then(Value::as_str)
            .and_then(|token| Token::ALL.iter().copied().find(|t| t.name() == token))
    };
    doc.get("lang")
        .and_then(Value::as_table)
        .map(|langs| {
            langs
                .iter()
                .filter_map(|(name, spec)| Some(lang(name, spec.as_table()?, &style)))
                .collect()
        })
        .unwrap_or_default()
}

fn strings(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn grouped(
    spec: &Table,
    key: &str,
    style: &impl Fn(&str) -> Option<Token>,
) -> Vec<(String, Token)> {
    spec.get(key)
        .and_then(Value::as_table)
        .into_iter()
        .flat_map(|groups| groups.iter())
        .filter_map(|(class, items)| Some((style(class)?, strings(Some(items)))))
        .flat_map(|(token, items)| items.into_iter().map(move |item| (item, token)))
        .collect()
}

fn longest_first(
    spec: &Table,
    key: &str,
    style: &impl Fn(&str) -> Option<Token>,
) -> Vec<(String, Token)> {
    let mut items = grouped(spec, key, style);
    items.sort_by_key(|(item, _)| std::cmp::Reverse(item.len()));
    items
}

fn lang(name: &str, spec: &Table, style: &impl Fn(&str) -> Option<Token>) -> Lang {
    let mut names = vec![name.to_owned()];
    names.extend(strings(spec.get("aliases")));
    let block = strings(spec.get("block_comment"));
    Lang {
        names,
        line_comments: strings(spec.get("line_comments")),
        block_comment: match block.as_slice() {
            [open, close] if !open.is_empty() && !close.is_empty() => {
                Some((open.clone(), close.clone()))
            }
            _ => None,
        },
        nested_block_comments: spec
            .get("nested_block_comments")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        strings: strings(spec.get("strings"))
            .iter()
            .filter_map(|quote| quote.chars().next())
            .collect(),
        numbers: spec
            .get("numbers")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        words: grouped(spec, "words", style).into_iter().collect(),
        prefixes: longest_first(spec, "prefixes", style),
        markers: longest_first(spec, "markers", style),
        comment: style("comment"),
        string: style("string"),
        number: style("number"),
    }
}

const fn is_word(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn scan(line: &str, from: usize, keep: impl Fn(char) -> bool) -> usize {
    line.get(from..)
        .and_then(|rest| rest.find(|ch| !keep(ch)).map(|at| from + at))
        .unwrap_or(line.len())
}

fn string_end(line: &str, from: usize, quote: char) -> usize {
    let mut escaped = false;
    for (at, ch) in line.get(from..).unwrap_or_default().char_indices() {
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == quote {
            return from + at + ch.len_utf8();
        }
    }
    line.len()
}

struct SpanMarks {
    spans: Vec<Highlight>,
    open: usize,
}

impl SpanMarks {
    fn push(&mut self, start: usize, end: usize, token: Option<Token>) -> usize {
        if let Some(token) = token.filter(|_| end > start) {
            self.spans.push(Highlight { start, end, token });
        }
        end
    }
}

enum Comment {
    Line,
    Block(usize),
}

impl Lang {
    fn comment_at(&self, rest: &str) -> Option<Comment> {
        let line = self
            .line_comments
            .iter()
            .filter(|mark| rest.starts_with(mark.as_str()))
            .map(String::len)
            .max();
        let block = self
            .block_comment
            .as_ref()
            .map(|(opener, _)| opener)
            .filter(|opener| rest.starts_with(opener.as_str()))
            .map(String::len);
        match (line, block) {
            (_, Some(opener)) if opener >= line.unwrap_or(0) => Some(Comment::Block(opener)),
            (Some(_), _) => Some(Comment::Line),
            _ => None,
        }
    }

    fn line(&self, line: &str, open: &mut usize) -> Vec<Highlight> {
        if let Some((_, token)) = self
            .prefixes
            .iter()
            .find(|(prefix, _)| line.starts_with(prefix.as_str()))
        {
            return vec![Highlight {
                start: 0,
                end: line.len(),
                token: *token,
            }];
        }
        let mut marks = SpanMarks {
            spans: Vec::new(),
            open: *open,
        };
        let mut i = self.marker(line, &mut marks);
        while let Some(ch) = line.get(i..).and_then(|rest| rest.chars().next()) {
            i = if marks.open > 0 {
                self.close_block(line, i, &mut marks)
            } else {
                self.token_at(line, i, ch, &mut marks)
            };
        }
        *open = marks.open;
        marks.spans
    }

    fn marker(&self, line: &str, marks: &mut SpanMarks) -> usize {
        if marks.open > 0 {
            return 0;
        }
        let indent = line.len() - line.trim_start().len();
        let rest = &line[indent..];
        self.markers
            .iter()
            .find(|(marker, _)| rest.starts_with(marker.as_str()))
            .map_or(0, |(marker, token)| {
                marks.push(indent, indent + marker.len(), Some(*token))
            })
    }

    fn close_block(&self, line: &str, i: usize, marks: &mut SpanMarks) -> usize {
        let Some((open, close)) = &self.block_comment else {
            marks.open = 0;
            return marks.push(i, line.len(), self.comment);
        };
        if !self.nested_block_comments {
            let found = line[i..].find(close);
            marks.open = usize::from(found.is_none());
            let end = found.map_or(line.len(), |at| i + at + close.len());
            return marks.push(i, end, self.comment);
        }
        let mut end = i;
        for (offset, _) in line[i..].char_indices() {
            let at = i + offset;
            if at < end {
                continue;
            }
            let rest = &line[at..];
            if rest.starts_with(close) {
                marks.open -= 1;
                end = at + close.len();
                if marks.open == 0 {
                    break;
                }
            } else if rest.starts_with(open) {
                marks.open = marks.open.saturating_add(1);
                end = at + open.len();
            }
        }
        if marks.open > 0 {
            end = line.len();
        }
        marks.push(i, end, self.comment)
    }

    fn token_at(&self, line: &str, i: usize, ch: char, marks: &mut SpanMarks) -> usize {
        match self.comment_at(&line[i..]) {
            Some(Comment::Line) => return marks.push(i, line.len(), self.comment),
            Some(Comment::Block(opener)) => {
                marks.open = 1;
                return marks.push(i, i + opener, self.comment);
            }
            None => {}
        }
        if self.strings.contains(&ch) {
            let end = string_end(line, i + ch.len_utf8(), ch);
            return marks.push(i, end, self.string);
        }
        if ch.is_ascii_digit() {
            let end = scan(line, i, |c| is_word(c) || c == '.');
            return marks.push(i, end, self.number.filter(|_| self.numbers));
        }
        if is_word(ch) {
            let end = scan(line, i, is_word);
            return marks.push(i, end, self.words.get(&line[i..end]).copied());
        }
        i + ch.len_utf8()
    }
}
