use super::parse::Inline;
use crate::app::HostCaps;
use crate::style::{TextFace, TextRun, Token};
use crate::widgets::text_search::NextMatch;

const MAX_MATH_BYTES: usize = 512;

fn find_from(next: &mut NextMatch, source: &str, from: usize) -> Option<usize> {
    next.find(source.get(from..)?).map(|at| at + from)
}

struct Inlines<'a> {
    source: &'a str,
    tick: NextMatch,
    star: NextMatch,
    double: NextMatch,
    bracket: NextMatch,
    paren: NextMatch,
}

impl Inlines<'_> {
    fn span_at(&mut self, i: usize) -> Option<(Inline, usize)> {
        let source = self.source;
        let rest = source.get(i..)?;
        if rest.starts_with('`')
            && let Some(end) = find_from(&mut self.tick, source, i + 1)
        {
            return Some((Inline::Code(source[i + 1..end].to_string()), end + 1));
        }
        let delimiters = if rest.starts_with("\\(") {
            Some(("\\(", "\\)"))
        } else if rest.starts_with("\\[") {
            Some(("\\[", "\\]"))
        } else if rest.starts_with("$$") {
            Some(("$$", "$$"))
        } else if rest.starts_with('$')
            && rest
                .chars()
                .nth(1)
                .is_some_and(|ch| !ch.is_whitespace() && !ch.is_ascii_digit())
        {
            Some(("$", "$"))
        } else {
            None
        };
        if let Some((open, close)) = delimiters {
            let body = rest.get(open.len()..)?;
            let mut limit = body.len().min(MAX_MATH_BYTES);
            while !body.is_char_boundary(limit) {
                limit -= 1;
            }
            if let Some(end) = body[..limit].find(close) {
                let expression = body[..end].trim();
                let valid_dollar = open != "$"
                    || (!body[..end].ends_with(char::is_whitespace)
                        && !body[end + close.len()..]
                            .chars()
                            .next()
                            .is_some_and(|ch| ch.is_alphanumeric() || ch == '_'));
                if !expression.is_empty() && valid_dollar {
                    let rendered = rust_latex_parser::to_unicode(
                        &rust_latex_parser::parse_equation(expression),
                    );
                    return Some((Inline::Text(rendered), i + open.len() + end + close.len()));
                }
            }
        }
        if rest.starts_with('[')
            && let Some((label, href, next)) = self.link_at(i)
        {
            return Some((link(label, href), next));
        }
        if let Some(url) = rest
            .strip_prefix('<')
            .and_then(|tail| tail.split_once('>'))
            .map(|(url, _)| url)
            .filter(|url| url.contains("://") && !url.contains(char::is_whitespace))
        {
            return Some((link(url.to_string(), url.to_string()), i + url.len() + 2));
        }
        if rest.starts_with("**")
            && opens(source, i + 2)
            && let Some(end) = find_from(&mut self.double, source, i + 2)
                .filter(|&end| end > i + 2 && closes(source, end))
        {
            return Some((Inline::Strong(parse_inlines(&source[i + 2..end])), end + 2));
        }
        if rest.starts_with('*')
            && opens(source, i + 1)
            && let Some(end) = find_from(&mut self.star, source, i + 1)
                .filter(|&end| end > i + 1 && closes(source, end))
        {
            return Some((Inline::Em(parse_inlines(&source[i + 1..end])), end + 1));
        }
        None
    }

    fn link_at(&mut self, i: usize) -> Option<(String, String, usize)> {
        let source = self.source;
        let close = find_from(&mut self.bracket, source, i + 1)?;
        let label = source[i + 1..close].to_string();
        source.get(close + 1..)?.strip_prefix('(')?;
        let end = find_from(&mut self.paren, source, close + 2)?;
        let url = source[close + 2..end].to_string();
        Some((label, url, end + 1))
    }
}

fn link(label: String, href: String) -> Inline {
    let label_is_href = label == href;
    let body = if label.contains("://") {
        vec![Inline::Text(label)]
    } else {
        parse_inlines(&label)
    };
    Inline::Link {
        href,
        body,
        label_is_href,
    }
}

pub(crate) fn parse_inlines(source: &str) -> Vec<Inline> {
    let mut scan = Inlines {
        source,
        tick: NextMatch::new("`", false),
        star: NextMatch::new("*", false),
        double: NextMatch::new("**", false),
        bracket: NextMatch::new("]", false),
        paren: NextMatch::new(")", false),
    };
    let mut out = Vec::new();
    let mut i = 0;
    let mut text = 0;
    while i < source.len() {
        if let Some((span, next)) = scan.span_at(i) {
            Inline::push_text(&mut out, &source[text..i]);
            out.push(span);
            i = next;
            text = next;
            continue;
        }
        i += source[i..].chars().next().map_or(1, char::len_utf8);
    }
    Inline::push_text(&mut out, source.get(text..).unwrap_or_default());
    Inline::finish(out)
}

fn opens(source: &str, at: usize) -> bool {
    source
        .get(at..)
        .and_then(|rest| rest.chars().next())
        .is_some_and(|ch| !ch.is_whitespace())
}

fn closes(source: &str, end: usize) -> bool {
    source
        .get(..end)
        .and_then(|head| head.chars().next_back())
        .is_some_and(|ch| !ch.is_whitespace())
}

#[derive(Default)]
pub(super) struct StyledText {
    pub(super) text: String,
    pub(super) runs: Vec<TextRun>,
}

pub(super) fn styled(body: &[Inline]) -> StyledText {
    let mut out = StyledText::default();
    push_spans(&mut out, body, &TextFace::default());
    out
}

fn push_spans(out: &mut StyledText, spans: &[Inline], face: &TextFace) {
    for span in spans {
        match span {
            Inline::Text(body) => push_run(out, body, face),
            Inline::Strong(body) => push_spans(
                out,
                body,
                &TextFace {
                    bold: true,
                    ..face.clone()
                },
            ),
            Inline::Em(body) => push_spans(
                out,
                body,
                &TextFace {
                    italic: true,
                    ..face.clone()
                },
            ),
            Inline::Code(body) => {
                let code = TextFace {
                    mono: true,
                    color: Some(Token::OnSurface),
                    background: Some(Token::SurfaceSubtle),
                    ..face.clone()
                };
                push_run(out, &unbreakable(body), &code);
            }
            Inline::Link {
                href,
                body,
                label_is_href,
            } => push_link(out, face, (body, *label_is_href), href),
        }
    }
}

fn push_link(
    out: &mut StyledText,
    face: &TextFace,
    (body, label_is_href): (&[Inline], bool),
    href: &str,
) {
    let link = TextFace {
        color: Some(Token::Accent11),
        link: Some(href.into()),
        ..face.clone()
    };
    match body {
        [Inline::Text(url)] if url.contains("://") => push_run(out, &url_breaks(url), &link),
        _ => push_spans(out, body, &link),
    }
    if !HostCaps::current().hyperlinks && !label_is_href {
        push_run(out, " (", face);
        let url = TextFace {
            color: Some(Token::OnSurfaceMuted),
            ..face.clone()
        };
        push_run(out, &url_breaks(href), &url);
        push_run(out, ")", face);
    }
}

const JOINER: char = '\u{2060}';
const BREAK: char = '\u{200b}';

fn url_breaks(url: &str) -> String {
    let host = url.find("://").map_or(0, |at| at + 3);
    let mut out = String::with_capacity(url.len() * 4);
    let mut last = None;
    for (index, ch) in url.char_indices() {
        if let Some(prev) = last {
            let open = index > host && matches!(prev, '/' | '?' | '&' | '=');
            out.push(if open { BREAK } else { JOINER });
        }
        out.push(ch);
        last = Some(ch);
    }
    out
}

const CODE_BREAKS: [char; 8] = ['/', ',', ';', '=', '&', '|', '(', '{'];

fn unbreakable(body: &str) -> String {
    let mut opened = String::with_capacity(body.len() + 8);
    let mut chars = body.chars().peekable();
    while let Some(ch) = chars.next() {
        opened.push(ch);
        if CODE_BREAKS.contains(&ch) && chars.peek().is_some_and(|next| !next.is_whitespace()) {
            opened.push(BREAK);
        }
    }
    let mut out = String::with_capacity(opened.len());
    let mut at = 0;
    for (index, _) in unicode_linebreak::linebreaks(&opened) {
        let Some(piece) = opened.get(at..index) else {
            continue;
        };
        out.push_str(piece);
        at = index;
        if index < opened.len() && !piece.ends_with(char::is_whitespace) && !piece.ends_with(BREAK)
        {
            out.push(JOINER);
        }
    }
    out
}

#[cfg(test)]
#[path = "inline_tests.rs"]
mod tests;

fn push_run(out: &mut StyledText, body: &str, face: &TextFace) {
    let start = out.text.len();
    out.text.push_str(body);
    if *face != TextFace::default() && !body.is_empty() {
        out.runs.push(TextRun {
            start,
            end: out.text.len(),
            face: face.clone(),
        });
    }
}
