use super::parse::Inline;
use crate::app::HostCaps;
use crate::style::{TextFace, TextRun, Token};

#[derive(Clone, Default)]
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
            Inline::Strong(body) => push_marked(out, body, face, |inner| inner.bold = true),
            Inline::Em(body) => push_marked(out, body, face, |inner| inner.italic = true),
            Inline::Strike(body) => push_marked(out, body, face, |inner| inner.strike = true),
            Inline::Code(body) => {
                let code = TextFace {
                    mono: true,
                    color: Some(Token::OnSurface),
                    background: Some(Token::SurfaceSubtle),
                    ..face.clone()
                };
                push_run(out, &unbreakable(body), &code);
            }
            Inline::Math(body) => push_run(out, &crate::widgets::math::unicode(body), face),
            Inline::Note(label) => push_note(out, label, face),
            Inline::Image { alt, .. } => {
                let muted = TextFace {
                    color: Some(Token::OnSurfaceMuted),
                    ..face.clone()
                };
                push_run(out, &format!("[{alt}]"), &muted);
            }
            Inline::Link {
                href,
                body,
                label_is_href,
            } => {
                push_link(out, face, (body, *label_is_href), href);
            }
        }
    }
}

fn push_marked(
    out: &mut StyledText,
    spans: &[Inline],
    face: &TextFace,
    mark: impl FnOnce(&mut TextFace),
) {
    let mut inner = face.clone();
    mark(&mut inner);
    push_spans(out, spans, &inner);
}

pub(super) fn note_mark(label: &str) -> String {
    const DIGITS: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];
    if label.bytes().all(|byte| byte.is_ascii_digit()) {
        label
            .bytes()
            .filter_map(|byte| DIGITS.get(usize::from(byte - b'0')))
            .collect()
    } else {
        format!("[{label}]")
    }
}

fn push_note(out: &mut StyledText, label: &str, face: &TextFace) {
    let mark = TextFace {
        color: Some(Token::Accent11),
        ..face.clone()
    };
    push_run(out, &note_mark(label), &mark);
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
