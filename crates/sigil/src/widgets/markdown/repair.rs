use std::borrow::Cow;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mark {
    Strong,
    Em,
    Strike,
    Bracket,
}

impl Mark {
    const fn closer(self) -> &'static str {
        match self {
            Self::Strong => "**",
            Self::Em => "*",
            Self::Strike => "~~",
            Self::Bracket => "",
        }
    }
}

#[derive(Default)]
struct Scan {
    stack: Vec<Mark>,
    inner: Option<String>,
}

pub(super) fn tail(source: &str) -> Cow<'_, str> {
    let line_start = source.rfind('\n').map_or(0, |at| at + 1);
    let body = source.trim_end();
    if body.len() <= line_start || in_fence(source) {
        return Cow::Borrowed(source);
    }
    let scan = scan(&body[line_start..]);
    let mut closers = scan.inner.unwrap_or_default();
    for mark in scan.stack.iter().rev() {
        closers.push_str(mark.closer());
    }
    if closers.is_empty() {
        return Cow::Borrowed(source);
    }
    let mut out = String::with_capacity(source.len() + closers.len());
    out.push_str(body);
    out.push_str(&closers);
    out.push_str(&source[body.len()..]);
    Cow::Owned(out)
}

fn in_fence(source: &str) -> bool {
    source
        .lines()
        .filter(|line| {
            let line = line.trim_start();
            line.starts_with("```") || line.starts_with("~~~")
        })
        .count()
        % 2
        == 1
}

fn scan(segment: &str) -> Scan {
    let bytes = segment.as_bytes();
    let mut out = Scan::default();
    let mut at = 0;
    while let Some(&byte) = bytes.get(at) {
        let step = match byte {
            b'`' => code(segment, at, &mut out),
            b'*' if bytes.get(at + 1) == Some(&b'*') => {
                Some(toggle(&mut out, segment, at, Mark::Strong))
            }
            b'*' => Some(toggle(&mut out, segment, at, Mark::Em)),
            b'~' if bytes.get(at + 1) == Some(&b'~') => {
                Some(toggle(&mut out, segment, at, Mark::Strike))
            }
            b'[' => {
                out.stack.push(Mark::Bracket);
                Some(1)
            }
            b']' => bracket_end(segment, at, &mut out),
            b'\\' => Some(2),
            _ => Some(1),
        };
        let Some(step) = step else {
            break;
        };
        at += step;
    }
    out
}

fn code(segment: &str, at: usize, out: &mut Scan) -> Option<usize> {
    let run = segment[at..]
        .bytes()
        .take_while(|byte| *byte == b'`')
        .count();
    let fence = &segment[at..at + run];
    let Some(end) = segment[at + run..].find(fence) else {
        out.inner = Some(fence.to_string());
        return None;
    };
    Some(run + end + run)
}

fn toggle(out: &mut Scan, segment: &str, at: usize, mark: Mark) -> usize {
    let width = mark.closer().len();
    let before = segment[..at].chars().next_back();
    let after = segment[at + width..].chars().next();
    if out.stack.last() == Some(&mark) && before.is_some_and(|ch| !ch.is_whitespace()) {
        out.stack.pop();
    } else if after.is_some_and(|ch| !ch.is_whitespace()) {
        out.stack.push(mark);
    }
    width
}

fn bracket_end(segment: &str, at: usize, out: &mut Scan) -> Option<usize> {
    let Some(open) = out.stack.iter().rposition(|mark| *mark == Mark::Bracket) else {
        return Some(1);
    };
    out.stack.truncate(open);
    if !segment[at + 1..].starts_with('(') {
        return Some(1);
    }
    let Some(end) = segment[at + 2..].find(')') else {
        out.inner = Some(String::from(")"));
        return None;
    };
    Some(end + 3)
}
