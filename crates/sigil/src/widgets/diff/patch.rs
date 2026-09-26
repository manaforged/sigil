use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Meta,
    Hunk,
    Context,
    Change,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Pair {
    pub(super) left: Option<String>,
    pub(super) right: Option<String>,
    pub(super) kind: Kind,
}

pub(super) fn pairs(patch: &str) -> Vec<Pair> {
    let mut rows = Vec::new();
    let mut removed: Vec<String> = Vec::new();
    let mut added: Vec<String> = Vec::new();
    for line in patch.lines() {
        if let Some(rest) = line.strip_prefix('-')
            && !line.starts_with("---")
        {
            removed.push(rest.to_string());
        } else if let Some(rest) = line.strip_prefix('+')
            && !line.starts_with("+++")
        {
            added.push(rest.to_string());
        } else {
            flush(&mut rows, &mut removed, &mut added);
            let kind = if line.starts_with("@@") {
                Kind::Hunk
            } else if let Some(rest) = line.strip_prefix(' ') {
                rows.push(context(rest));
                continue;
            } else {
                Kind::Meta
            };
            rows.push(Pair {
                left: Some(line.to_string()),
                right: None,
                kind,
            });
        }
    }
    flush(&mut rows, &mut removed, &mut added);
    rows
}

fn context(line: &str) -> Pair {
    Pair {
        left: Some(line.to_string()),
        right: Some(line.to_string()),
        kind: Kind::Context,
    }
}

fn flush(out: &mut Vec<Pair>, dropped: &mut Vec<String>, kept: &mut Vec<String>) {
    while !dropped.is_empty() && dropped.first() == kept.first() {
        let line = dropped.remove(0);
        kept.remove(0);
        out.push(context(&line));
    }
    let mut tail = Vec::new();
    while !dropped.is_empty() && dropped.last() == kept.last() {
        tail.push(context(&dropped.pop().unwrap_or_default()));
        kept.pop();
    }
    for index in 0..dropped.len().max(kept.len()) {
        out.push(Pair {
            left: dropped.get(index).cloned(),
            right: kept.get(index).cloned(),
            kind: Kind::Change,
        });
    }
    out.extend(tail.into_iter().rev());
    dropped.clear();
    kept.clear();
}

pub(super) fn starts(line: &str) -> Option<(usize, usize)> {
    let mut parts = line.strip_prefix("@@ ")?.split_whitespace();
    let old = parts
        .next()?
        .strip_prefix('-')?
        .split(',')
        .next()?
        .parse()
        .ok()?;
    let new = parts
        .next()?
        .strip_prefix('+')?
        .split(',')
        .next()?
        .parse()
        .ok()?;
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
