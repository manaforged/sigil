use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use std::hash::{Hash as _, Hasher as _};

use crate::geometry::{Point, Rect};
use crate::kernel::{Node, NodeId, NodeType, TextContent};
use crate::style::text_identity::DiscreteTextStyle;
use crate::style::{Rgba8, StyleBinding, TextStyle, Theme};
use ahash::AHasher;

use crate::scene::builder::SceneBuilder;
use crate::scene::command::SceneCommand;

const CAP: usize = 4_096;

struct Entry {
    version: u32,
    slot: u8,
    rect: [u32; 4],
    inherit: u64,
    cmds: Vec<SceneCommand>,
}

struct Layered {
    base: Rc<TextStyle>,
    atoms: u64,
    style: Rc<TextStyle>,
}

thread_local! {
    static CACHE: RefCell<HashMap<(u64, NodeId), Entry>> = RefCell::new(HashMap::new());
    static LAYERED: RefCell<HashMap<u64, Layered>> = RefCell::new(HashMap::new());
    static THEME_EPOCH: Cell<u64> = const { Cell::new(0) };
}

pub(crate) fn layered_text(base: &Rc<TextStyle>, binding: &StyleBinding) -> Rc<TextStyle> {
    let atoms = typography_key(binding);
    let key = DiscreteTextStyle::identity(base) ^ atoms;
    let hit = LAYERED.with_borrow(|cache| {
        cache
            .get(&key)
            .filter(|entry| {
                entry.atoms == atoms && (Rc::ptr_eq(&entry.base, base) || *entry.base == **base)
            })
            .map(|entry| Rc::clone(&entry.style))
    });
    if let Some(style) = hit {
        return style;
    }
    let style =
        Rc::new(TextStyle::layered_over_binding(base, binding).unwrap_or_else(|| (**base).clone()));
    LAYERED.with_borrow_mut(|cache| {
        if cache.len() >= CAP {
            cache.clear();
        }
        cache.insert(
            key,
            Layered {
                base: Rc::clone(base),
                atoms,
                style: Rc::clone(&style),
            },
        );
    });
    style
}

fn typography_key(binding: &StyleBinding) -> u64 {
    let mut hasher = AHasher::default();
    for atom in binding
        .cascade()
        .filter(|atom| crate::style::is_typography_atom(*atom))
    {
        atom.hash(&mut hasher);
    }
    if let Some(overlay) = binding.animator_overlay() {
        for kind in [
            crate::style::NumericAtomKind::FontSize,
            crate::style::NumericAtomKind::LineHeight,
        ] {
            overlay.value_for(kind).map(f32::to_bits).hash(&mut hasher);
        }
    }
    hasher.finish()
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
            || entry.rect[2..] != rect_key(rect)[2..]
        {
            return false;
        }
        crate::profile::count(crate::profile::Counter::EmitReplays);
        let dx = rect.origin.x - f32::from_bits(entry.rect[0]);
        let dy = rect.origin.y - f32::from_bits(entry.rect[1]);
        if dx != 0.0 || dy != 0.0 {
            for command in &mut entry.cmds {
                shift_command(command, dx, dy);
            }
            entry.rect = rect_key(rect);
        }
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
    #[allow(clippy::useless_let_if_seq, reason = "intentional, reviewed trade-off")]
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
            overlay.cache_key().hash(&mut hasher);
            wrote = true;
        }
    }
    if !wrote {
        return 0;
    }
    hasher.finish()
}

pub(crate) fn theme_changed() {
    THEME_EPOCH.with(|epoch| epoch.set(epoch.get().wrapping_add(1)));
}

pub(crate) fn inherit_key(theme: &Theme, text_color: Option<Rgba8>, text_style: &TextStyle) -> u64 {
    let theme_p = std::ptr::from_ref(theme) as u64 ^ THEME_EPOCH.with(Cell::get).rotate_left(37);
    let color = text_color.map_or(0, |c| u64::from(u32::from_le_bytes([c.r, c.g, c.b, c.a])));
    let size = u64::from(text_style.font_size.to_bits());
    let weight = u64::from(text_style.font_weight);
    theme_p ^ color.rotate_left(7) ^ size.rotate_left(17) ^ weight.rotate_left(27)
}

fn slot_tag(node: &Node) -> u8 {
    node.get_extension::<StyleBinding>()
        .map_or(0, |binding| match binding.interaction_slot() {
            crate::style::StateSlot::Rest => 0,
            crate::style::StateSlot::Focused => 1,
            crate::style::StateSlot::Hovered => 2,
            crate::style::StateSlot::Pressed => 3,
            crate::style::StateSlot::Disabled => 4,
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
        SceneCommand::Rect(draw) => {
            draw.rect = shift_rect(draw.rect, dx, dy);
            if let Some(border) = draw.border.as_mut() {
                border.rect = shift_rect(border.rect, dx, dy);
            }
        }
        SceneCommand::Gradient(draw) => draw.rect = shift_rect(draw.rect, dx, dy),
        SceneCommand::Pattern(draw) => draw.rect = shift_rect(draw.rect, dx, dy),
        SceneCommand::GlyphRun(draw) => draw.run.rect = shift_rect(draw.run.rect, dx, dy),
        SceneCommand::Image(draw) => draw.destination = shift_rect(draw.destination, dx, dy),
        SceneCommand::Sprite(draw) => draw.destination = shift_rect(draw.destination, dx, dy),
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
