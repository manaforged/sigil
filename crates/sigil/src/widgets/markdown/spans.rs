use std::ops::Range;

use super::events::Builder;
use super::parse::Inline;

const MAX_SPANS: usize = 32;
const MAX_MATH_BYTES: usize = 512;

pub(super) enum SpanKind {
    Base,
    Strong,
    Em,
    Strike,
    Link(String),
    Image(String),
}

pub(super) struct Span {
    kind: SpanKind,
    pub(super) body: Vec<Inline>,
}

impl Span {
    pub(super) const fn base() -> Self {
        Self {
            kind: SpanKind::Base,
            body: Vec::new(),
        }
    }
}

impl Builder<'_> {
    pub(super) fn open_span(&mut self, kind: SpanKind) {
        if self.spans.len() >= MAX_SPANS {
            self.flat_spans += 1;
            return;
        }
        self.spans.push(Span {
            kind,
            body: Vec::new(),
        });
    }

    pub(super) fn close_span(&mut self) {
        if self.flat_spans > 0 {
            self.flat_spans -= 1;
            return;
        }
        let Some(Span { kind, body }) = self.spans.pop() else {
            return;
        };
        let inline = match kind {
            SpanKind::Base => {
                self.spans.push(Span {
                    kind: SpanKind::Base,
                    body,
                });
                return;
            }
            SpanKind::Strong => Inline::Strong(body),
            SpanKind::Em => Inline::Em(body),
            SpanKind::Strike => Inline::Strike(body),
            SpanKind::Link(href) => Inline::Link {
                label_is_href: Inline::plain(&body) == href,
                href,
                body,
            },
            SpanKind::Image(src) => Inline::Image {
                alt: Inline::plain(&body),
                src,
            },
        };
        self.push_inline(inline);
    }

    pub(super) fn close_base(&mut self) -> Vec<Inline> {
        let mut out = Vec::new();
        while let Some(Span { kind, body }) = self.spans.pop() {
            let mut body = body;
            body.append(&mut out);
            out = body;
            if matches!(kind, SpanKind::Base) {
                break;
            }
        }
        linkify(out)
    }

    pub(super) fn math(&mut self, body: &str, range: Range<usize>) {
        let raw = self.source.get(range.clone()).unwrap_or_default();
        let braced = raw.starts_with("${") && raw.ends_with("}$");
        let dollar = raw.starts_with('$') && !raw.starts_with("$$") && !braced;
        let glued = self
            .source
            .get(range.end..)
            .and_then(|rest| rest.chars().next())
            .is_some_and(|ch| ch.is_alphanumeric() || ch == '_');
        let expression = body.trim();
        let expression = if braced {
            expression
                .strip_prefix('{')
                .and_then(|inner| inner.strip_suffix('}'))
                .unwrap_or(expression)
        } else {
            expression
        };
        if expression.is_empty() || expression.len() > MAX_MATH_BYTES || (dollar && glued) {
            self.push_text(raw);
            return;
        }
        self.push_inline(Inline::Math(expression.to_string()));
    }
}

pub(super) fn linkify(body: Vec<Inline>) -> Vec<Inline> {
    let mut out = Vec::with_capacity(body.len());
    for inline in body {
        match inline {
            Inline::Text(text) => split_urls(&mut out, &text),
            Inline::Strong(inner) => out.push(Inline::Strong(linkify(inner))),
            Inline::Em(inner) => out.push(Inline::Em(linkify(inner))),
            Inline::Strike(inner) => out.push(Inline::Strike(linkify(inner))),
            other => out.push(other),
        }
    }
    out
}

fn split_urls(out: &mut Vec<Inline>, text: &str) {
    let mut rest = text;
    while let Some((start, end)) = next_url(rest) {
        Inline::push_text(out, &rest[..start]);
        let url = &rest[start..end];
        let href = if url.starts_with("www.") {
            format!("http://{url}")
        } else {
            url.to_string()
        };
        out.push(Inline::Link {
            label_is_href: href == url,
            href,
            body: vec![Inline::Text(url.to_string())],
        });
        rest = &rest[end..];
    }
    Inline::push_text(out, rest);
}

fn next_url(text: &str) -> Option<(usize, usize)> {
    let start = ["https://", "http://", "www."]
        .iter()
        .filter_map(|scheme| {
            text.match_indices(scheme)
                .map(|(at, _)| at)
                .find(|&at| text[..at].chars().next_back().is_none_or(url_may_follow))
        })
        .min()?;
    let tail = &text[start..];
    let mut end = tail
        .find(|ch: char| ch.is_whitespace() || ch == '<')
        .unwrap_or(tail.len());
    loop {
        let url = &tail[..end];
        let unbalanced =
            url.ends_with(')') && url.matches('(').count() < url.matches(')').count();
        if url.ends_with(['.', ',', ':', ';', '!', '?', '\'', '"']) || unbalanced {
            end -= 1;
        } else {
            break;
        }
    }
    let url = &tail[..end];
    let host = url.split_once("://").map_or(url, |(_, host)| host);
    (host.contains('.') && host.len() > 3).then_some((start, start + end))
}

fn url_may_follow(ch: char) -> bool {
    !ch.is_alphanumeric() && ch != '/' && ch != '@' && ch != '.'
}
