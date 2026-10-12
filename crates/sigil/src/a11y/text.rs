use accesskit::{Action, Node as AkNode, NodeId as AkNodeId, Role, TextPosition, TextSelection};
use unicode_segmentation::UnicodeSegmentation;

use crate::kernel::{A11yProps, NodeId};

pub(super) type RunNodes = Vec<(AkNodeId, AkNode)>;

struct Run<'a> {
    id: AkNodeId,
    start: usize,
    text: &'a str,
}

pub(super) fn child_id(owner: NodeId, index: usize, kind: u64) -> AkNodeId {
    let index = u64::try_from(index).unwrap_or_default();
    let tag = index.wrapping_mul(2).wrapping_add(kind).wrapping_shl(32);
    AkNodeId(owner.to_accesskit().0 ^ tag)
}

fn runs(owner: NodeId, text: &str) -> Vec<Run<'_>> {
    let mut out = Vec::new();
    let mut start = 0;
    for (line, piece) in text.split_inclusive('\n').enumerate() {
        out.push(Run {
            id: child_id(owner, line, 1),
            start,
            text: piece,
        });
        start += piece.len();
    }
    if out.is_empty() || text.ends_with('\n') {
        out.push(Run {
            id: child_id(owner, out.len(), 1),
            start,
            text: "",
        });
    }
    out
}

fn position(runs: &[Run<'_>], byte: usize) -> Option<TextPosition> {
    let run = runs
        .iter()
        .find(|run| byte < run.start + run.text.len())
        .or_else(|| runs.last())?;
    let end = byte.saturating_sub(run.start).min(run.text.len());
    let prefix = run.text.get(..end)?;
    Some(TextPosition {
        node: run.id,
        character_index: prefix.graphemes(true).count(),
    })
}

fn byte_of(runs: &[Run<'_>], position: TextPosition) -> Option<usize> {
    let run = runs.iter().find(|run| run.id == position.node)?;
    let offset: usize = run
        .text
        .graphemes(true)
        .take(position.character_index)
        .map(str::len)
        .sum();
    Some(run.start + offset)
}

fn run_node(run: &Run<'_>) -> AkNode {
    let mut node = AkNode::new(Role::TextRun);
    node.set_value(run.text);
    let lengths: Vec<u8> = run
        .text
        .graphemes(true)
        .map(|grapheme| u8::try_from(grapheme.len()).unwrap_or(u8::MAX))
        .collect();
    node.set_character_lengths(lengths);
    node
}

pub(super) fn export(owner: NodeId, a11y: &A11yProps, ak: &mut AkNode) -> Option<RunNodes> {
    let (Some(value), Some(cursor)) = (a11y.text_value, a11y.text_cursor) else {
        return None;
    };
    let text = value.peek();
    let runs = runs(owner, &text);
    let focus = position(&runs, cursor.caret.peek())?;
    let anchor = cursor
        .anchor
        .peek()
        .and_then(|byte| position(&runs, byte))
        .unwrap_or(focus);
    ak.set_text_selection(TextSelection { anchor, focus });
    ak.add_action(Action::SetTextSelection);
    for run in &runs {
        ak.push_child(run.id);
    }
    Some(runs.iter().map(|run| (run.id, run_node(run))).collect())
}

pub(super) fn selection_bytes(
    owner: NodeId,
    a11y: &A11yProps,
    selection: TextSelection,
) -> Option<(usize, usize)> {
    let text = a11y.text_value?.peek();
    let runs = runs(owner, &text);
    Some((
        byte_of(&runs, selection.anchor)?,
        byte_of(&runs, selection.focus)?,
    ))
}
