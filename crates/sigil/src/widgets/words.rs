use unicode_segmentation::UnicodeSegmentation;

use crate::kernel::{Cx, IntoNode, NodeBuilder, NodeId, Role, container, text};
use crate::layout::{AlignItems, FlexWrap, LayoutStyle};
use crate::widgets::{Framed, Widget, WidgetFrame};

/// Builder of a line of text built from one node per word.
#[derive(Debug)]
#[must_use]
pub struct WordsBuilder {
    label: String,
    pieces: Vec<NodeBuilder>,
    frame: WidgetFrame,
}

/// Text that `piece` renders word by word in a wrapping row, read as one name; the `u32` index scales a `Duration` for a stagger, and each word is shaped alone, so kerning and complex-script shaping across words are lost.
pub fn words<N: Into<NodeBuilder>>(
    source: impl Into<String>,
    mut piece: impl FnMut(u32, &str) -> N,
) -> WordsBuilder {
    let label = source.into();
    let pieces = (0_u32..)
        .zip(split(&label))
        .map(|(index, (word, gap))| {
            spaced(piece(index, word).into(), gap).accessibility_hidden(true)
        })
        .collect();
    WordsBuilder {
        label,
        pieces,
        frame: WidgetFrame::new(),
    }
}

impl WordsBuilder {
    fn root(self) -> NodeBuilder {
        let line = container()
            .role(Role::Label)
            .accessible(self.label)
            .layout(
                LayoutStyle::flex_row()
                    .flex_wrap(FlexWrap::Wrap)
                    .align_items(AlignItems::Baseline),
            )
            .children(self.pieces);
        self.frame.finish(line)
    }
}

impl Framed for WordsBuilder {
    fn frame_mut(&mut self) -> &mut WidgetFrame {
        &mut self.frame
    }
}

impl IntoNode for WordsBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.root().build(cx)
    }
}

fn spaced(word: NodeBuilder, gap: &str) -> NodeBuilder {
    if gap.is_empty() {
        return word;
    }
    container()
        .layout(LayoutStyle::flex_row().align_items(AlignItems::Baseline))
        .child(word)
        .child(text(gap).nowrap())
}

fn split(source: &str) -> Vec<(&str, &str)> {
    let line = source.trim();
    let mut out = Vec::new();
    let (mut word_start, mut gap_start) = (0, None);
    for (at, fragment) in line.split_word_bound_indices() {
        let blank = fragment.chars().all(char::is_whitespace);
        match (blank, gap_start) {
            (true, None) => gap_start = Some(at),
            (false, Some(gap)) => {
                out.push((slice(line, word_start, gap), slice(line, gap, at)));
                word_start = at;
                gap_start = None;
            }
            _ => {}
        }
    }
    if !line.is_empty() {
        out.push((slice(line, word_start, line.len()), ""));
    }
    out
}

fn slice(line: &str, start: usize, end: usize) -> &str {
    line.get(start..end).unwrap_or_default()
}
