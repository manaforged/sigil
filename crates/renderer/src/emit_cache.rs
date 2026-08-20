//! Per-node paint command cache for unchanged nodes.

use std::cell::RefCell;
use std::collections::HashMap;

use std::hash::{Hash as _, Hasher as _};

use ahash::AHasher;
use sigil_hal::{Point, Rect};
use sigil_kernel::{Node, NodeId, NodeType, TextContent};
use sigil_style::{ColorAtomKind, NumericAtomKind, Rgba8, StyleBinding, TextStyle, Theme};

use crate::scene_builder::SceneBuilder;
use crate::scene_command::SceneCommand;

const CAP: usize = 4_096;

struct Entry {
    version: u32,
    slot: u8,
    rect: [u32; 4],
    inherit: u64,
    cmds: Vec<SceneCommand>,
}

thread_local! {
    static CACHE: RefCell<HashMap<(u64, NodeId), Entry>> = RefCell::new(HashMap::new());
}

pub(crate) fn replay(
    scene: &mut SceneBuilder,
    key: (u64, NodeId),
    node: &Node,
    rect: Rect,
    inherit: u64,
) -> bool {
    CACHE.with(|cell| {
        let mut cache = cell.borrow_mut();
        let Some(entry) = cache.get_mut(&key) else {
            return false;
        };
        if entry.version != node.version()
            || entry.slot != slot_tag(node)
            || entry.inherit != inherit
            || !same_size(entry.rect, rect)
        {
            return false;
        }
        let (dx, dy) = origin_delta(entry.rect, rect);
        if dx == 0.0 && dy == 0.0 {
            scene.extend_commands(entry.cmds.iter().cloned());
            return true;
        }
        for command in &mut entry.cmds {
            shift_command(command, dx, dy);
        }
        entry.rect = rect_key(rect);
        scene.extend_commands(entry.cmds.iter().cloned());
        true
    })
}

pub(crate) fn store(
    key: (u64, NodeId),
    node: &Node,
    rect: Rect,
    inherit: u64,
    cmds: Vec<SceneCommand>,
) {
    CACHE.with(|cell| {
        let mut cache = cell.borrow_mut();
        if cache.len() >= CAP && !cache.contains_key(&key) {
            cache.clear();
        }
        cache.insert(
            key,
            Entry {
                version: node.version(),
                slot: slot_tag(node),
                rect: rect_key(rect),
                inherit,
                cmds,
            },
        );
    });
}

pub(crate) fn content_key(node: &Node) -> u64 {
    let mut hasher = AHasher::default();
    // Set independently by each of the three hash sources below; the
    // lint's single-if rewrite would drop the later sources.
    #[allow(clippy::useless_let_if_seq)]
    let mut wrote = false;
    if let NodeType::Text { content, .. } = &node.kind
        && !matches!(content, TextContent::Static(_))
    {
        content.resolve().hash(&mut hasher);
        wrote = true;
    }
    if let Some(binding) = node.get_extension::<StyleBinding>() {
        for slice in binding.reactive_channels() {
            slice.hash(&mut hasher);
            wrote = true;
        }
        if let Some(overlay) = binding.animator_overlay() {
            hash_overlay(overlay, &mut hasher);
            wrote = true;
        }
    }
    if !wrote {
        return 0;
    }
    hasher.finish()
}

fn hash_overlay(overlay: &dyn sigil_style::AnimatorOverlay, hasher: &mut AHasher) {
    const KINDS: [NumericAtomKind; 7] = [
        NumericAtomKind::Opacity,
        NumericAtomKind::BorderRadius,
        NumericAtomKind::FontSize,
        NumericAtomKind::LineHeight,
        NumericAtomKind::Gap,
        NumericAtomKind::PaddingX,
        NumericAtomKind::PaddingY,
    ];
    for kind in KINDS {
        if let Some(value) = overlay.value_for(kind) {
            kind.hash(hasher);
            value.to_bits().hash(hasher);
        }
    }
    for kind in [ColorAtomKind::Background, ColorAtomKind::Text] {
        if let Some(color) = overlay.value_for_color(kind) {
            kind.hash(hasher);
            [color.r, color.g, color.b, color.a].hash(hasher);
        }
    }
}

pub(crate) fn inherit_key(theme: &Theme, text_color: Option<Rgba8>, text_style: &TextStyle) -> u64 {
    let theme_p = std::ptr::from_ref(theme) as u64;
    let color = text_color.map_or(0, |c| u64::from(u32::from_le_bytes([c.r, c.g, c.b, c.a])));
    let size = u64::from(text_style.font_size.to_bits());
    let weight = u64::from(text_style.font_weight);
    theme_p ^ color.rotate_left(7) ^ size.rotate_left(17) ^ weight.rotate_left(27)
}

fn slot_tag(node: &Node) -> u8 {
    node.get_extension::<StyleBinding>()
        .map_or(0, |binding| match binding.interaction_slot() {
            sigil_style::StateSlot::Rest => 0,
            sigil_style::StateSlot::Focused => 1,
            sigil_style::StateSlot::Hovered => 2,
            sigil_style::StateSlot::Pressed => 3,
            sigil_style::StateSlot::Disabled => 4,
        })
}

const fn rect_key(rect: Rect) -> [u32; 4] {
    [
        rect.origin.x.to_bits(),
        rect.origin.y.to_bits(),
        rect.width.to_bits(),
        rect.height.to_bits(),
    ]
}

const fn same_size(stored: [u32; 4], rect: Rect) -> bool {
    stored[2] == rect.width.to_bits() && stored[3] == rect.height.to_bits()
}

fn origin_delta(stored: [u32; 4], rect: Rect) -> (f32, f32) {
    (
        rect.origin.x - f32::from_bits(stored[0]),
        rect.origin.y - f32::from_bits(stored[1]),
    )
}

fn shift_rect(rect: Rect, dx: f32, dy: f32) -> Rect {
    Rect::new(
        Point::new(rect.origin.x + dx, rect.origin.y + dy),
        rect.width,
        rect.height,
    )
}

fn shift_command(command: &mut SceneCommand, dx: f32, dy: f32) {
    match command {
        SceneCommand::PushClip(rect) => *rect = shift_rect(*rect, dx, dy),
        SceneCommand::Rect(draw) => draw.rect = shift_rect(draw.rect, dx, dy),
        SceneCommand::Gradient(draw) => draw.rect = shift_rect(draw.rect, dx, dy),
        SceneCommand::Pattern(draw) => draw.rect = shift_rect(draw.rect, dx, dy),
        SceneCommand::GlyphRun(draw) => draw.run.rect = shift_rect(draw.run.rect, dx, dy),
        SceneCommand::Image(draw) => {
            draw.destination = shift_rect(draw.destination, dx, dy);
        }
        SceneCommand::Sprite(draw) => {
            draw.destination = shift_rect(draw.destination, dx, dy);
        }
        SceneCommand::Decoration(draw) => draw.band = shift_rect(draw.band, dx, dy),
        SceneCommand::DrawShadow { rect, .. } => {
            rect[0] += dx;
            rect[1] += dy;
        }
        SceneCommand::PushLayer(scope) => scope.clip = shift_rect(scope.clip, dx, dy),
        SceneCommand::PopClip
        | SceneCommand::PushTransform(_)
        | SceneCommand::PopTransform
        | SceneCommand::SaveLayer { .. }
        | SceneCommand::RestoreLayer
        | SceneCommand::PopLayer => {}
    }
}
