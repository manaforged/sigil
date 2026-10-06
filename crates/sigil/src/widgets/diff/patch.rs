use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LineKind {
    Context,
    Removed,
    Added,
    Note,
}

impl LineKind {
    #[must_use]
    pub const fn is_change(self) -> bool {
        matches!(self, Self::Removed | Self::Added)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LineId {
    pub hunk: usize,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub kind: LineKind,
    pub text: String,
    pub old: Option<usize>,
    pub new: Option<usize>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Hunk {
    pub meta: Vec<String>,
    pub path: Option<String>,
    pub header: String,
    pub old_start: usize,
    pub new_start: usize,
    pub lines: Vec<Line>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Patch {
    pub hunks: Vec<Hunk>,
    pub trailer: Vec<String>,
}

impl Patch {
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let mut parser = Parser::default();
        for line in text.lines() {
            parser.feed(line);
        }
        parser.patch.trailer = parser.meta;
        parser.patch
    }

    #[must_use]
    pub fn line(&self, id: LineId) -> Option<&Line> {
        self.hunks.get(id.hunk)?.lines.get(id.line)
    }

    pub fn changes(&self) -> impl Iterator<Item = LineId> + '_ {
        (0..self.hunks.len()).flat_map(|hunk| self.hunk_changes(hunk))
    }

    pub fn hunk_changes(&self, hunk: usize) -> impl Iterator<Item = LineId> + '_ {
        self.hunks
            .get(hunk)
            .into_iter()
            .flat_map(|item| item.lines.iter().enumerate())
            .filter(|(_, line)| line.kind.is_change())
            .map(move |(line, _)| LineId { hunk, line })
    }
}

#[derive(Default)]
struct Parser {
    patch: Patch,
    meta: Vec<String>,
    path: Option<String>,
    old: usize,
    new: usize,
    open: bool,
}

impl Parser {
    fn feed(&mut self, line: &str) {
        if line.starts_with("@@") {
            let (old, new) = starts(line).unwrap_or((1, 1));
            self.start(line.to_string(), old, new);
            return;
        }
        if !line.starts_with("---")
            && !line.starts_with("+++")
            && let Some((kind, text)) = body(line, self.open)
        {
            if !self.open {
                self.start(String::new(), 1, 1);
            }
            self.push(kind, text);
            return;
        }
        self.open = false;
        self.track_path(line);
        self.meta.push(line.to_string());
    }

    fn start(&mut self, header: String, old: usize, new: usize) {
        self.patch.hunks.push(Hunk {
            meta: std::mem::take(&mut self.meta),
            path: self.path.clone(),
            header,
            old_start: old,
            new_start: new,
            lines: Vec::new(),
        });
        (self.old, self.new, self.open) = (old, new, true);
    }

    fn push(&mut self, kind: LineKind, text: &str) {
        let old = matches!(kind, LineKind::Context | LineKind::Removed).then_some(self.old);
        let new = matches!(kind, LineKind::Context | LineKind::Added).then_some(self.new);
        self.old += usize::from(old.is_some());
        self.new += usize::from(new.is_some());
        if let Some(hunk) = self.patch.hunks.last_mut() {
            hunk.lines.push(Line {
                kind,
                text: text.to_string(),
                old,
                new,
            });
        }
    }

    fn track_path(&mut self, line: &str) {
        if line.starts_with("diff ") {
            self.path = None;
        }
        let named = line
            .strip_prefix("--- ")
            .or_else(|| line.strip_prefix("+++ "))
            .and_then(|rest| rest.split('\t').next())
            .filter(|name| *name != "/dev/null");
        if let Some(name) = named {
            let name = name
                .strip_prefix("a/")
                .or_else(|| name.strip_prefix("b/"))
                .unwrap_or(name);
            self.path = Some(name.to_string());
        }
    }
}

fn body(line: &str, open: bool) -> Option<(LineKind, &str)> {
    let mut chars = line.chars();
    let kind = match chars.next() {
        Some('-') => LineKind::Removed,
        Some('+') => LineKind::Added,
        Some(' ') => LineKind::Context,
        Some('\\') if open => return Some((LineKind::Note, line)),
        None if open => return Some((LineKind::Context, "")),
        _ => return None,
    };
    Some((kind, chars.as_str()))
}

pub(super) fn starts(line: &str) -> Option<(usize, usize)> {
    let mut parts = line.strip_prefix("@@ ")?.split_whitespace();
    let mut start = |sign: char| -> Option<usize> {
        parts
            .next()?
            .strip_prefix(sign)?
            .split(',')
            .next()?
            .parse()
            .ok()
    };
    let old = start('-')?;
    let new = start('+')?;
    Some((old, new))
}

pub(super) fn changed(left: &str, right: &str) -> (Range<usize>, Range<usize>) {
    let prefix = left
        .chars()
        .zip(right.chars())
        .take_while(|(a, b)| a == b)
        .map(|(c, _)| c.len_utf8())
        .sum::<usize>();
    let suffix = left[prefix..]
        .chars()
        .rev()
        .zip(right[prefix..].chars().rev())
        .take_while(|(a, b)| a == b)
        .map(|(c, _)| c.len_utf8())
        .sum::<usize>();
    (prefix..left.len() - suffix, prefix..right.len() - suffix)
}
