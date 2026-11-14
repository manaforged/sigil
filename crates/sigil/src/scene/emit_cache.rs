use std::cell::{Cell, RefCell};
use std::rc::Rc;

use std::hash::{Hash as _, Hasher as _};

use crate::cache::Bounded;
use crate::fnv::{OFFSET, mix, mix_f32};
use crate::geometry::{Point, Rect};
use crate::kernel::{Node, NodeId, NodeType, TextContent};
use crate::style::text_identity::DiscreteTextStyle;
use crate::style::{Color, StyleBinding, TextStyle, Theme};
use ahash::AHasher;

use crate::scene::builder::SceneBuilder;
use crate::scene::command::{LayerEffect, LayerScope, SceneCommand};
use crate::scene::shadow::Shadow;

const CAP: usize = 4_096;

struct EmitEntry {
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
    static CACHE: RefCell<Bounded<(u64, NodeId), EmitEntry, CAP>> = RefCell::new(Bounded::default());
    static LAYERED: RefCell<Bounded<u64, Layered, CAP>> = RefCell::new(Bounded::default());
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
            || entry.rect[2..] != rect_bits(rect)[2..]
        {
            return false;
        }
        crate::profile::count(crate::profile::Counter::EmitReplays, 1);
        let dx = rect.origin.x - f32::from_bits(entry.rect[0]);
        let dy = rect.origin.y - f32::from_bits(entry.rect[1]);
        if dx != 0.0 || dy != 0.0 {
            for command in &mut entry.cmds {
                shift_command(command, dx, dy);
            }
            entry.rect = rect_bits(rect);
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
        cache.insert(
            key,
            EmitEntry {
                version: node.version(),
                slot: slot_tag(node),
                rect: rect_bits(rect),
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
        content.hash_into(&mut hasher);
        wrote = true;
    }
    if let Some(binding) = node.extension::<StyleBinding>() {
        for slice in binding.reactive_channels() {
            slice.hash(&mut hasher);
            wrote = true;
        }
        if let Some(overlay) = binding.animator_overlay() {
            let key = overlay.cache_key();
            if key != 0 {
                key.hash(&mut hasher);
                wrote = true;
            }
        }
    }
    if !wrote {
        return 0;
    }
    hasher.finish()
}

pub(crate) fn theme_epoch() -> u64 {
    THEME_EPOCH.with(Cell::get)
}

pub(crate) fn theme_changed() {
    THEME_EPOCH.with(|epoch| epoch.set(epoch.get().wrapping_add(1)));
}

pub(crate) fn theme_stamp() -> u64 {
    mix(OFFSET, theme_epoch())
}

pub(crate) fn theme_key(theme: &Theme, stamp: u64) -> u64 {
    mix(mix(OFFSET, std::ptr::from_ref(theme) as u64), stamp)
}

pub(crate) fn inherit_key(theme: &Theme, text_color: Option<Color>, text_style: &TextStyle) -> u64 {
    let color = text_color.map_or(0, crate::fnv::rgba);
    let key = mix(theme_key(theme, theme_stamp()), color);
    mix(
        mix_f32(key, text_style.font_size),
        u64::from(text_style.font_weight),
    )
}

/// A key that changes when any inherited text shadow changes.
pub(crate) fn shadow_key(shadows: &[Shadow]) -> u64 {
    shadows.iter().fold(0, |key, shadow| {
        let Shadow {
            color,
            offset,
            blur,
            spread,
            ..
        } = *shadow;
        color
            .into_iter()
            .chain(offset)
            .chain([blur, spread])
            .fold(key, mix_f32)
    })
}

pub(crate) fn slot_tag(node: &Node) -> u8 {
    node.extension::<StyleBinding>()
        .map_or(0, |binding| match binding.interaction_slot() {
            crate::style::StateSlot::Rest => 0,
            crate::style::StateSlot::Focused => 1,
            crate::style::StateSlot::Hovered => 2,
            crate::style::StateSlot::Pressed => 3,
            crate::style::StateSlot::Disabled => 4,
        })
}

pub(crate) const fn rect_bits(rect: Rect) -> [u32; 4] {
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

pub(crate) fn shift_command(command: &mut SceneCommand, dx: f32, dy: f32) {
    match command {
        SceneCommand::PushClip(clip) => clip.shift(dx, dy),
        SceneCommand::Rect(draw) => {
            draw.rect = shift_rect(draw.rect, dx, dy);
            if let Some(border) = draw.border.as_mut() {
                border.rect = shift_rect(border.rect, dx, dy);
            }
        }
        SceneCommand::Gradient(draw) => {
            draw.rect = shift_rect(draw.rect, dx, dy);
            draw.gradient.shift(dx, dy);
        }
        SceneCommand::Pattern(draw) => draw.rect = shift_rect(draw.rect, dx, dy),
        SceneCommand::GlyphRun(draw) => draw.shift(dx, dy),
        SceneCommand::Image(draw) => draw.destination = shift_rect(draw.destination, dx, dy),
        #[cfg(feature = "kittest")]
        SceneCommand::Sprite(draw) => draw.destination = shift_rect(draw.destination, dx, dy),
        SceneCommand::Decoration(draw) => draw.band = shift_rect(draw.band, dx, dy),
        SceneCommand::Shader(draw) => draw.rect = shift_rect(draw.rect, dx, dy),
        SceneCommand::Path(draw) => draw.shift(dx, dy),
        SceneCommand::DrawShadow { rect, .. } => {
            rect[0] += dx;
            rect[1] += dy;
        }
        SceneCommand::PushLayer(scope) => shift_layer(scope, dx, dy),
        SceneCommand::PopClip | SceneCommand::PopLayer => {}
        SceneCommand::PushTransform(_) | SceneCommand::PopTransform => {}
        #[cfg(host)]
        SceneCommand::SaveLayer { .. } => {}
        #[cfg(terminal_host)]
        SceneCommand::RestoreLayer => {}
    }
}

fn shift_layer(scope: &mut LayerScope, dx: f32, dy: f32) {
    scope.clip = shift_rect(scope.clip, dx, dy);
    if let LayerEffect::Mask(mask) = &mut scope.effect {
        let mut commands = mask.commands.to_vec();
        for command in &mut commands {
            shift_command(command, dx, dy);
        }
        mask.commands = commands.into();
    }
}
