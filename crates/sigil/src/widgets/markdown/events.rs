use std::ops::Range;

use pulldown_cmark::{BlockQuoteKind, CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

use super::chart_parse::fence_block;
use super::containers::{List, ListItem, TaskMark};
use super::details;
use super::parse::{Block, Inline, Note};
use super::spans::{Span, SpanKind, linkify};
use super::table_data::{Align, Cell, Table};

pub(super) type Located = (Range<usize>, Block);

const MAX_NESTING: usize = 16;

pub(super) const OPTIONS: Options = Options::ENABLE_TABLES
    .union(Options::ENABLE_TASKLISTS)
    .union(Options::ENABLE_FOOTNOTES)
    .union(Options::ENABLE_STRIKETHROUGH)
    .union(Options::ENABLE_GFM)
    .union(Options::ENABLE_MATH);

enum Container {
    Root,
    Quote(Option<BlockQuoteKind>),
    List(Option<u64>, Vec<ListItem>),
    Item(Option<TaskMark>),
    Note(String),
    Table(Vec<Align>, Vec<Vec<Cell>>, Vec<Cell>),
}

struct Frame {
    kind: Container,
    blocks: Vec<Block>,
    loose: Vec<Inline>,
}

impl Frame {
    const fn new(kind: Container) -> Self {
        Self {
            kind,
            blocks: Vec::new(),
            loose: Vec::new(),
        }
    }
}

enum Leaf {
    Code(String, String),
    Html(String),
}

pub(super) struct Builder<'s> {
    pub(super) source: &'s str,
    frames: Vec<Frame>,
    pub(super) spans: Vec<Span>,
    pub(super) flat_spans: usize,
    leaf: Option<Leaf>,
    flat: usize,
    top: Vec<Located>,
}

pub(super) fn located(source: &str) -> Vec<Located> {
    let mut builder = Builder {
        source,
        frames: vec![Frame::new(Container::Root)],
        spans: Vec::new(),
        flat_spans: 0,
        leaf: None,
        flat: 0,
        top: Vec::new(),
    };
    for (event, range) in Parser::new_ext(source, OPTIONS).into_offset_iter() {
        builder.event(event, range);
    }
    details::fold(builder.top)
}

impl Builder<'_> {
    fn event(&mut self, event: Event<'_>, range: Range<usize>) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag, range),
            Event::Text(body) => self.text(&body),
            Event::Html(body) => self.html(&body),
            Event::InlineHtml(body) => self.push_text(&body),
            Event::Code(body) => self.push_inline(Inline::Code(body.into_string())),
            Event::InlineMath(body) | Event::DisplayMath(body) => self.math(&body, range),
            Event::FootnoteReference(label) => {
                self.push_inline(Inline::Note(label.into_string()));
            }
            Event::SoftBreak => self.push_text(" "),
            Event::HardBreak => self.push_text("\n"),
            Event::Rule => self.push_block(Block::Rule, range),
            Event::TaskListMarker(done) => self.task(done),
        }
    }

    fn start(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Paragraph | Tag::Heading { .. } | Tag::TableCell => self.spans.push(Span::base()),
            Tag::BlockQuote(kind) => self.open(Container::Quote(kind)),
            Tag::List(start) => self.open(Container::List(start, Vec::new())),
            Tag::Item => self.open(Container::Item(None)),
            Tag::FootnoteDefinition(label) => self.open(Container::Note(label.into_string())),
            Tag::Table(align) => {
                let align = align.into_iter().map(Align::from_delimiter).collect();
                self.open(Container::Table(align, Vec::new(), Vec::new()));
            }
            Tag::CodeBlock(kind) => {
                let lang = match kind {
                    CodeBlockKind::Fenced(info) => info.split_whitespace().next().unwrap_or_default().to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                self.leaf = Some(Leaf::Code(lang, String::new()));
            }
            Tag::HtmlBlock => self.leaf = Some(Leaf::Html(String::new())),
            Tag::Emphasis => self.open_span(SpanKind::Em),
            Tag::Strong => self.open_span(SpanKind::Strong),
            Tag::Strikethrough => self.open_span(SpanKind::Strike),
            Tag::Link { dest_url, .. } => self.open_span(SpanKind::Link(dest_url.into_string())),
            Tag::Image { dest_url, .. } => self.open_span(SpanKind::Image(dest_url.into_string())),
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd, range: Range<usize>) {
        match tag {
            TagEnd::Paragraph => {
                let body = self.close_base();
                self.push_block(paragraph(body), range);
            }
            TagEnd::Heading(level) => {
                let body = Inline::finish(self.close_base());
                self.push_block(Block::Heading { level: level as u8, body }, range);
            }
            TagEnd::TableCell => self.cell(),
            TagEnd::TableHead | TagEnd::TableRow => self.row(),
            TagEnd::CodeBlock | TagEnd::HtmlBlock => self.close_leaf(range),
            TagEnd::Emphasis
            | TagEnd::Strong
            | TagEnd::Strikethrough
            | TagEnd::Link
            | TagEnd::Image => self.close_span(),
            TagEnd::BlockQuote(_)
            | TagEnd::List(_)
            | TagEnd::Item
            | TagEnd::FootnoteDefinition
            | TagEnd::Table => self.close(range),
            _ => {}
        }
    }

    fn text(&mut self, body: &str) {
        match self.leaf.as_mut() {
            Some(Leaf::Code(_, buf) | Leaf::Html(buf)) => buf.push_str(body),
            None => self.push_text(body),
        }
    }

    fn html(&mut self, body: &str) {
        match self.leaf.as_mut() {
            Some(Leaf::Html(buf) | Leaf::Code(_, buf)) => buf.push_str(body),
            None => self.push_text(body),
        }
    }

    pub(super) fn push_inline(&mut self, inline: Inline) {
        let target = match self.spans.last_mut() {
            Some(span) => &mut span.body,
            None => match self.frames.last_mut() {
                Some(frame) => &mut frame.loose,
                None => return,
            },
        };
        match inline {
            Inline::Text(text) => Inline::push_text(target, &text),
            other => target.push(other),
        }
    }

    pub(super) fn push_text(&mut self, text: &str) {
        self.push_inline(Inline::Text(text.to_string()));
    }

    fn task(&mut self, done: bool) {
        let mark = if done { TaskMark::Done } else { TaskMark::Open };
        if let Some(Frame {
            kind: Container::Item(task),
            ..
        }) = self.frames.last_mut()
        {
            *task = Some(mark);
        }
    }

    fn push_block(&mut self, block: Block, range: Range<usize>) {
        self.flush_loose();
        match self.frames.as_mut_slice() {
            [_] => self.top.push((range, block)),
            [.., frame] => frame.blocks.push(block),
            [] => {}
        }
    }

    fn flush_loose(&mut self) {
        let Some(frame) = self.frames.last_mut() else {
            return;
        };
        if frame.loose.is_empty() {
            return;
        }
        let body = linkify(std::mem::take(&mut frame.loose));
        frame.blocks.push(paragraph(body));
    }

    fn open(&mut self, kind: Container) {
        self.flush_loose();
        if self.frames.len() >= MAX_NESTING {
            self.flat += 1;
            return;
        }
        self.frames.push(Frame::new(kind));
    }

    fn close(&mut self, range: Range<usize>) {
        if self.flat > 0 {
            self.flat -= 1;
            return;
        }
        self.flush_loose();
        if self.frames.len() < 2 {
            return;
        }
        let Some(frame) = self.frames.pop() else {
            return;
        };
        let blocks = details::fold_blocks(frame.blocks);
        let block = match frame.kind {
            Container::Root => return,
            Container::Quote(None) => Block::Quote(blocks),
            Container::Quote(Some(kind)) => Block::Alert { kind, blocks },
            Container::List(start, items) => Block::List(List { start, items }),
            Container::Item(task) => return self.push_item(ListItem { task, blocks }, range),
            Container::Note(label) => Block::Footnotes(vec![Note { label, blocks }]),
            Container::Table(align, rows, _) => Block::Table(Table { align, rows }),
        };
        self.push_block(block, range);
    }

    fn push_item(&mut self, item: ListItem, range: Range<usize>) {
        if let Some(Frame {
            kind: Container::List(_, items),
            ..
        }) = self.frames.last_mut()
        {
            items.push(item);
            return;
        }
        for block in item.blocks {
            self.push_block(block, range.clone());
        }
    }

    fn cell(&mut self) {
        let body = Inline::finish(self.close_base());
        match self.frames.last_mut() {
            Some(Frame {
                kind: Container::Table(_, _, row),
                ..
            }) => row.push(body),
            _ => self.push_block(Block::Paragraph(body), 0..0),
        }
    }

    fn row(&mut self) {
        if let Some(Frame {
            kind: Container::Table(_, rows, row),
            ..
        }) = self.frames.last_mut()
        {
            rows.push(std::mem::take(row));
        }
    }

    fn close_leaf(&mut self, range: Range<usize>) {
        let block = match self.leaf.take() {
            Some(Leaf::Code(lang, body)) => {
                let body = body.strip_suffix('\n').unwrap_or(&body).to_string();
                fence_block(&lang, body)
            }
            Some(Leaf::Html(body)) => Block::Html(body),
            None => return,
        };
        self.push_block(block, range);
    }
}

fn paragraph(body: Vec<Inline>) -> Block {
    let mut content = body
        .iter()
        .filter(|inline| !matches!(inline, Inline::Text(text) if text.trim().is_empty()));
    let only = match (content.next(), content.next()) {
        (Some(first), None) => Some(first),
        _ => None,
    };
    match only {
        Some(Inline::Math(source)) => Block::Math(source.clone()),
        Some(Inline::Image { alt, src }) => Block::Image {
            alt: alt.clone(),
            src: src.clone(),
        },
        _ => Block::Paragraph(Inline::finish(body)),
    }
}
