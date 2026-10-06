use std::collections::HashMap;
use std::ops::Range;
use std::sync::LazyLock;

use toml::{Table, Value};

use super::HlSpan;
use super::grapheme::col_count;
use crate::style::{TextFace, TextRun, Token};

const LANGUAGES: &str = include_str!("highlight/languages.toml");

static CATALOG: LazyLock<Vec<Lang>> = LazyLock::new(|| catalog(LANGUAGES));

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Highlight {
    pub start: usize,
    pub end: usize,
    pub token: Token,
}

#[derive(Default)]
struct Classes {
    comment: Option<Token>,
    string: Option<Token>,
    number: Option<Token>,
}

#[derive(Default)]
struct Lang {
    names: Vec<String>,
    line_comments: Vec<String>,
    block_comment: Option<(String, String)>,
    strings: Vec<char>,
    numbers: bool,
    words: HashMap<String, Token>,
    prefixes: Vec<(String, Token)>,
    markers: Vec<(String, Token)>,
    classes: Classes,
}

#[must_use]
pub fn highlight(language: &str, source: &str) -> Vec<Vec<Highlight>> {
    let tag = language.trim().to_ascii_lowercase();
    let lang = CATALOG.iter().find(|lang| lang.names.contains(&tag));
    let mut open = false;
    source
        .split('\n')
        .map(|line| lang.map_or_else(Vec::new, |lang| lang.line(line, &mut open)))
        .collect()
}

#[must_use]
pub fn language_for_path(path: &str) -> Option<&'static str> {
    let name = path.rsplit(['/', '\\']).next()?.to_ascii_lowercase();
    let tag = name.rsplit_once('.').map_or(name.as_str(), |(_, ext)| ext);
    CATALOG
        .iter()
        .find(|lang| lang.names.iter().any(|known| known == tag))
        .and_then(|lang| lang.names.first())
        .map(String::as_str)
}

#[must_use]
pub fn text_runs(marks: &[Highlight], range: Range<usize>) -> Vec<TextRun> {
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

#[must_use]
pub fn hl_spans(language: &str, source: &str) -> Vec<Vec<HlSpan>> {
    source
        .split('\n')
        .zip(highlight(language, source))
        .map(|(line, marks)| {
            let col = |byte: usize| line.get(..byte).map_or(0, col_count);
            marks
                .into_iter()
                .map(|mark| HlSpan {
                    start: col(mark.start),
                    end: col(mark.end),
                    token: mark.token.name(),
                })
                .collect()
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
            [open, close] => Some((open.clone(), close.clone())),
            _ => None,
        },
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
        classes: Classes {
            comment: style("comment"),
            string: style("string"),
            number: style("number"),
        },
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

struct Marks {
    spans: Vec<Highlight>,
    open: bool,
}

impl Marks {
    fn push(&mut self, start: usize, end: usize, token: Option<Token>) -> usize {
        if let Some(token) = token.filter(|_| end > start) {
            self.spans.push(Highlight { start, end, token });
        }
        end
    }
}

impl Lang {
    fn line(&self, line: &str, open: &mut bool) -> Vec<Highlight> {
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
        let mut marks = Marks {
            spans: Vec::new(),
            open: *open,
        };
        let mut i = self.marker(line, &mut marks);
        while let Some(ch) = line.get(i..).and_then(|rest| rest.chars().next()) {
            i = if marks.open {
                self.close_block(line, i, &mut marks)
            } else {
                self.token_at(line, i, ch, &mut marks)
            };
        }
        *open = marks.open;
        marks.spans
    }

    fn marker(&self, line: &str, marks: &mut Marks) -> usize {
        if marks.open {
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

    fn close_block(&self, line: &str, i: usize, marks: &mut Marks) -> usize {
        let close = self.block_comment.as_ref().map_or("", |(_, close)| close);
        let found = line[i..].find(close).filter(|_| !close.is_empty());
        marks.open = found.is_none();
        let end = found.map_or(line.len(), |at| i + at + close.len());
        marks.push(i, end, self.classes.comment)
    }

    fn token_at(&self, line: &str, i: usize, ch: char, marks: &mut Marks) -> usize {
        let rest = &line[i..];
        if self.line_comments.iter().any(|mark| rest.starts_with(mark)) {
            return marks.push(i, line.len(), self.classes.comment);
        }
        if let Some((opener, _)) = self
            .block_comment
            .as_ref()
            .filter(|(opener, _)| rest.starts_with(opener.as_str()))
        {
            marks.open = true;
            return marks.push(i, i + opener.len(), self.classes.comment);
        }
        if self.strings.contains(&ch) {
            let end = string_end(line, i + ch.len_utf8(), ch);
            return marks.push(i, end, self.classes.string);
        }
        if ch.is_ascii_digit() {
            let end = scan(line, i, |c| is_word(c) || c == '.');
            return marks.push(i, end, self.classes.number.filter(|_| self.numbers));
        }
        if is_word(ch) {
            let end = scan(line, i, is_word);
            return marks.push(i, end, self.words.get(&line[i..end]).copied());
        }
        i + ch.len_utf8()
    }
}
