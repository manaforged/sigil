use super::parse::Span;
use crate::app::HostCaps;
use crate::style::{TextFace, TextRun, Token};
use crate::widgets::text_search::NextMatch;

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
    fn span_at(&mut self, i: usize) -> Option<(Span, usize)> {
        let source = self.source;
        let rest = source.get(i..)?;
        if rest.starts_with('`')
            && let Some(end) = find_from(&mut self.tick, source, i + 1)
        {
            return Some((Span::Code(source[i + 1..end].to_string()), end + 1));
        }
        if rest.starts_with('[')
            && let Some((label, href, next)) = self.link_at(i)
        {
            return Some((Span::Link { text: label, href }, next));
        }
        if let Some(url) = rest
            .strip_prefix('<')
            .and_then(|tail| tail.split_once('>'))
            .map(|(url, _)| url)
            .filter(|url| url.contains("://") && !url.contains(char::is_whitespace))
        {
            let span = Span::Link {
                text: url.to_string(),
                href: url.to_string(),
            };
            return Some((span, i + url.len() + 2));
        }
        if rest.starts_with("**")
            && let Some(end) = find_from(&mut self.double, source, i + 2)
        {
            return Some((Span::Bold(source[i + 2..end].to_string()), end + 2));
        }
        if rest.starts_with('*')
            && let Some(end) = find_from(&mut self.star, source, i + 1)
        {
            return Some((Span::Italic(source[i + 1..end].to_string()), end + 1));
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

pub(crate) fn parse_inlines(source: &str) -> Vec<Span> {
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
    let mut text = String::new();
    while i < source.len() {
        if let Some((span, next)) = scan.span_at(i) {
            flush_text(&mut out, &mut text);
            out.push(span);
            i = next;
            continue;
        }
        let ch = source[i..].chars().next().unwrap_or(' ');
        text.push(ch);
        i += ch.len_utf8();
    }
    flush_text(&mut out, &mut text);
    if out.is_empty() {
        out.push(Span::Text(String::new()));
    }
    out
}

fn flush_text(out: &mut Vec<Span>, text: &mut String) {
    if !text.is_empty() {
        out.push(Span::Text(std::mem::take(text)));
    }
}

#[derive(Default)]
pub(super) struct Styled {
    pub(super) text: String,
    pub(super) runs: Vec<TextRun>,
}

pub(super) fn styled(source: &str) -> Styled {
    let mut out = Styled::default();
    push_spans(&mut out, source, &TextFace::default());
    out
}

fn push_spans(out: &mut Styled, source: &str, face: &TextFace) {
    for span in parse_inlines(source) {
        match span {
            Span::Text(body) => push_run(out, &body, face),
            Span::Bold(body) => push_spans(
                out,
                &body,
                &TextFace {
                    bold: true,
                    ..face.clone()
                },
            ),
            Span::Italic(body) => {
                push_spans(
                    out,
                    &body,
                    &TextFace {
                        italic: true,
                        ..face.clone()
                    },
                );
            }
            Span::Code(body) => {
                let code = TextFace {
                    mono: true,
                    color: Some(Token::OnSurface),
                    background: Some(Token::SurfaceSubtle),
                    ..face.clone()
                };
                push_run(out, &unbreakable(&body), &code);
            }
            Span::Link { text, href } => push_link(out, face, &text, &href),
        }
    }
}

fn push_link(out: &mut Styled, face: &TextFace, text: &str, href: &str) {
    let link = TextFace {
        color: Some(Token::Accent11),
        link: Some(href.into()),
        ..face.clone()
    };
    if text.contains("://") {
        push_run(out, &url_breaks(text), &link);
    } else {
        push_spans(out, text, &link);
    }
    if !HostCaps::current().hyperlinks && text != href {
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

fn unbreakable(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut at = 0;
    for (index, _) in unicode_linebreak::linebreaks(body) {
        let Some(piece) = body.get(at..index) else {
            continue;
        };
        out.push_str(piece);
        at = index;
        if index < body.len() && !piece.ends_with(char::is_whitespace) {
            out.push(JOINER);
        }
    }
    out
}

fn push_run(out: &mut Styled, body: &str, face: &TextFace) {
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
