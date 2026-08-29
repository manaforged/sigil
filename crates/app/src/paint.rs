use std::rc::Rc;

use sigil_hal::Rect;
use sigil_kernel::{Cx, Node, NodeId, NodeType, Surface, TextContent};
use sigil_layout::LayoutResult;
use sigil_style::{
    Rgba8, StateSlot, StyleAtom, StyleBinding, TextStyle, Theme, Tone, ToneContext,
    render_style_for,
};

use crate::Painted;

#[derive(Clone)]
pub(crate) struct Resolved {
    id: NodeId,
    version: u32,
    ink_in: Option<Rgba8>,
    text_in: Rc<TextStyle>,
    slices: Slices,
    slot_in: StateSlot,
    fill: Option<Rgba8>,
    ink: Option<Rgba8>,
    text: Rc<TextStyle>,
}

pub(crate) struct Cascade<'a> {
    theme: &'a Theme,
    clip: Option<Rect>,
    ink: Option<Rgba8>,
    text: Rc<TextStyle>,
}

impl<'a> Cascade<'a> {
    pub(crate) fn root(theme: &'a Theme, text: &Rc<TextStyle>) -> Self {
        Self {
            theme,
            clip: None,
            ink: None,
            text: Rc::clone(text),
        }
    }
}

fn store(
    out: &mut Vec<Painted>,
    index: usize,
    node: Painted,
    content: Option<&TextContent>,
    kept: bool,
) -> bool {
    let Some(slot) = out.get_mut(index) else {
        let mut fresh = node;
        fresh.text = content.map(TextContent::resolve);
        out.push(fresh);
        return true;
    };
    let text = slot.text.take();
    let same = kept && slot.id == node.id && slot.rect == node.rect && slot.clip == node.clip;
    let mut node = node;
    node.bare = slot.bare;
    *slot = node;
    let (text, verbatim) = match (content, text) {
        (Some(content), Some(mut buffer)) => {
            let changed = content.refresh(&mut buffer);
            (Some(buffer), !changed)
        }
        (Some(content), None) => (Some(content.resolve()), false),
        (None, _) => (None, true),
    };
    slot.text = text;
    !(same && verbatim)
}

pub(crate) struct Walk<'a> {
    pub(crate) cx: &'a Cx,
    pub(crate) cache: &'a mut Vec<Resolved>,
    pub(crate) out: &'a mut Vec<Painted>,
    pub(crate) next: &'a mut usize,
    pub(crate) moved: &'a mut bool,
}

pub(crate) fn fill_painted(walk: &mut Walk<'_>, id: NodeId, inherited: &Cascade<'_>) {
    let Some(node) = walk.cx.get(id) else {
        return;
    };
    if node.get_extension::<Surface>().is_some() {
        return;
    }
    let theme = node.get_extension::<Theme>().unwrap_or(inherited.theme);
    let binding = node.get_extension::<StyleBinding>();
    let tone = node.get_extension::<ToneContext>().map(|ctx| ctx.tone);
    let slot = *walk.next;
    let ask = Ask {
        id,
        theme,
        binding,
        ink: inherited.ink,
        text: &inherited.text,
    };
    let laid = node.get_extension::<LayoutResult>();
    let (style, kept) = styled(walk, slot, node, &ask, laid.is_some());
    let mut down = Cascade {
        theme,
        clip: inherited.clip,
        ink: style.ink.or(inherited.ink),
        text: Rc::clone(&style.text),
    };
    let Some(result) = laid else {
        for child in node.children() {
            fill_painted(walk, *child, &down);
        }
        return;
    };
    let rect = result.rect();
    *walk.next = walk.next.saturating_add(1);
    if store(
        walk.out,
        slot,
        mark(id, rect, tone, &style, inherited.clip),
        text_of(node),
        kept,
    ) {
        *walk.moved = true;
    }
    if result.clips() {
        down.clip = Some(inherited.clip.map_or(rect, |outer| {
            outer
                .intersection(rect)
                .unwrap_or_else(|| Rect::new(rect.origin, 0.0, 0.0))
        }));
    }
    descend(walk, node, slot, &down);
}

fn descend(walk: &mut Walk<'_>, node: &Node, slot: usize, down: &Cascade<'_>) {
    let before = *walk.next;
    for child in node.children() {
        fill_painted(walk, *child, down);
    }
    let bare = *walk.next == before;
    if let Some(painted) = walk.out.get_mut(slot)
        && painted.bare != bare
    {
        painted.bare = bare;
        *walk.moved = true;
    }
}

const fn text_of(node: &Node) -> Option<&TextContent> {
    match &node.kind {
        NodeType::Text { content, .. } => Some(content),
        NodeType::Container | NodeType::Image { .. } | NodeType::Fragment => None,
    }
}

fn mark(
    id: NodeId,
    rect: Rect,
    tone: Option<Tone>,
    style: &Resolved,
    clip: Option<Rect>,
) -> Painted {
    Painted {
        id,
        rect,
        text: None,
        tone,
        fill: style.fill,
        ink: style.ink,
        weight: style.text.font_weight,
        bare: true,
        clip,
    }
}

#[derive(Clone, Copy)]
struct Slices {
    seen: [(usize, usize); Self::ROOM],
    count: usize,
    spilled: bool,
}

impl Slices {
    const ROOM: usize = 6;

    fn of(binding: Option<&StyleBinding>) -> Self {
        let mut seen = [(0, 0); Self::ROOM];
        let mut count = 0;
        let mut spilled = false;
        let mut note = |atoms: &[StyleAtom]| {
            if let Some(slot) = seen.get_mut(count) {
                *slot = (atoms.as_ptr() as usize, atoms.len());
                count += 1;
            } else {
                spilled = true;
            }
        };
        if let Some(binding) = binding {
            note(binding.dynamic_slice());
            for atoms in binding.reactive_channels() {
                note(atoms);
            }
            note(binding.state_overlay_atoms());
        }
        Self {
            seen,
            count,
            spilled,
        }
    }

    fn still(&self, binding: Option<&StyleBinding>) -> bool {
        if self.spilled {
            return false;
        }
        let mut at = 0;
        let mut ok = true;
        let mut check = |atoms: &[StyleAtom]| {
            let want = (atoms.as_ptr() as usize, atoms.len());
            ok &= self.seen.get(at).is_some_and(|seen| *seen == want) && at < self.count;
            at += 1;
        };
        if let Some(binding) = binding {
            check(binding.dynamic_slice());
            for atoms in binding.reactive_channels() {
                check(atoms);
            }
            check(binding.state_overlay_atoms());
        }
        ok && at == self.count
    }
}

struct Ask<'a> {
    id: NodeId,
    theme: &'a Theme,
    binding: Option<&'a StyleBinding>,
    ink: Option<Rgba8>,
    text: &'a Rc<TextStyle>,
}

impl Ask<'_> {
    fn slot(&self) -> StateSlot {
        self.binding
            .map(StyleBinding::interaction_slot)
            .unwrap_or_default()
    }
}

fn resolve(walk: &mut Walk<'_>, slot: usize, node: &Node, ask: &Ask<'_>) -> (Resolved, bool) {
    let version = node.version();
    let state = ask.slot();
    let animating = ask.binding.is_some_and(|b| b.animator_overlay().is_some());
    if !animating
        && let Some(hit) = walk.cache.get(slot)
        && hit.id == ask.id
        && hit.version == version
        && hit.ink_in == ask.ink
        && hit.slot_in == state
        && hit.slices.still(ask.binding)
        && Rc::ptr_eq(&hit.text_in, ask.text)
    {
        return (hit.clone(), true);
    }
    let fresh = compute(node, ask, version, state, Slices::of(ask.binding));
    match walk.cache.get_mut(slot) {
        Some(entry) => *entry = fresh.clone(),
        None => walk.cache.push(fresh.clone()),
    }
    (fresh, false)
}

fn styled(
    walk: &mut Walk<'_>,
    slot: usize,
    node: &Node,
    ask: &Ask<'_>,
    owns_slot: bool,
) -> (Resolved, bool) {
    if owns_slot {
        return resolve(walk, slot, node, ask);
    }
    let slices = Slices::of(ask.binding);
    (
        compute(node, ask, node.version(), ask.slot(), slices),
        false,
    )
}

fn compute(node: &Node, ask: &Ask<'_>, version: u32, state: StateSlot, slices: Slices) -> Resolved {
    let style = render_style_for(ask.id, node, ask.theme, ask.ink);
    let layered = ask
        .binding
        .and_then(|binding| TextStyle::layered_over_binding(ask.text, binding));
    Resolved {
        id: ask.id,
        version,
        ink_in: ask.ink,
        text_in: Rc::clone(ask.text),
        slices,
        slot_in: state,
        fill: style.background_color,
        ink: style.text_color,
        text: layered.map_or_else(|| Rc::clone(ask.text), Rc::new),
    }
}
