use std::cell::{Cell, RefCell};
use std::rc::Rc;

use smallvec::SmallVec;

use super::builder_ext::EMPTY_ATOMS;
use super::class_parse::intern_atoms;
use super::install::install_reactive;
use super::{Paint, StateStyleBuilder, StyleAtom, StyleBinding};
use crate::animation::runtime::registry::transition_config;
use crate::animation::runtime::{AnimationConfig, MotionPrefs, SignalAnimator};
use crate::animation::{Easing, Property};
use crate::kernel::{Cx, Node, NodeId};
use crate::layout::Display;
use crate::reactive::{Runtime, Signal, batch, effect_named, signal};

const NEXT_FRAME: AnimationConfig = AnimationConfig::Tween {
    ms: 1,
    easing: Easing::Linear,
    delay_ms: 0,
};

thread_local! {
    static RELEASED: RefCell<Vec<(u64, NodeId)>> = const { RefCell::new(Vec::new()) };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Edge {
    Enter,
    Exit,
}

impl Edge {
    const fn path(self) -> &'static str {
        match self {
            Self::Enter => "Widget::enter",
            Self::Exit => "Widget::exit",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Entering,
    Shown,
    Leaving,
    Hidden,
    Removing { cx: u64, id: NodeId },
}

impl Phase {
    const fn interactive(self) -> bool {
        matches!(self, Self::Entering | Self::Shown)
    }
}

struct Clock {
    progress: Signal<f32>,
    animator: RefCell<SignalAnimator>,
    started: Cell<Option<Phase>>,
}

impl Clock {
    fn new() -> Self {
        let progress = signal(0.0_f32);
        Self {
            progress,
            animator: RefCell::new(SignalAnimator::new(progress, NEXT_FRAME)),
            started: Cell::new(None),
        }
    }

    fn start(&self, phase: Phase, timing: Option<AnimationConfig>) {
        self.started.set(Some(phase));
        let mut animator = self.animator.borrow_mut();
        animator.snap(0.0);
        let config = match (phase, timing) {
            (Phase::Shown | Phase::Hidden, _) => return,
            (Phase::Entering, _) => NEXT_FRAME,
            (_, Some(config)) => config,
            (_, None) => {
                animator.snap(1.0);
                return;
            }
        };
        animator.set_config(config);
        animator.target(1.0);
    }

    fn done(&self, phase: Phase) -> bool {
        self.progress.get() >= 1.0
            && self.started.get() == Some(phase)
            && self.animator.borrow().settled()
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Presence {
    phase: Signal<Phase>,
    enter: Signal<&'static [StyleAtom]>,
    exit: Signal<&'static [StyleAtom]>,
    timing: Signal<Option<AnimationConfig>>,
}

impl Presence {
    pub(crate) fn interactive(self) -> bool {
        self.phase.try_peek().is_some_and(Phase::interactive)
    }

    fn of(node: &mut Node) -> Self {
        node.extension::<Self>()
            .copied()
            .unwrap_or_else(|| Self::install(node))
    }

    fn install(node: &mut Node) -> Self {
        let presence = Self {
            phase: signal(Phase::Shown),
            enter: signal(EMPTY_ATOMS),
            exit: signal(EMPTY_ATOMS),
            timing: signal(None),
        };
        let clock = Rc::new(Clock::new());
        presence.drive(Rc::clone(&clock));
        presence.release_when_done(Rc::clone(&clock));
        install_reactive(node, move || presence.atoms(&clock));
        if let Some(tag) = node.tag() {
            tag.queue_edit(move |tagged| presence.resolve(tagged));
        }
        let _previous = node.attach(presence);
        presence
    }

    fn drive(self, clock: Rc<Clock>) {
        effect_named("sigil-style::presence", move || {
            let phase = self.phase.get();
            let timing = self.timing.peek();
            batch(|| clock.start(phase, timing));
        });
    }

    fn release_when_done(self, clock: Rc<Clock>) {
        effect_named("sigil-style::presence-release", move || {
            let phase = self.phase.get();
            if let Phase::Removing { cx, id } = phase
                && clock.done(phase)
            {
                release(cx, id);
            }
        });
    }

    fn follow(self, shown: Signal<bool>) {
        effect_named("sigil-style::presence-shown", move || {
            let next = match (shown.get(), self.phase.peek()) {
                (true, Phase::Leaving | Phase::Hidden) => Phase::Shown,
                (false, Phase::Entering | Phase::Shown) => Phase::Leaving,
                _ => return,
            };
            self.phase.set(next);
        });
    }

    fn atoms(self, clock: &Clock) -> &'static [StyleAtom] {
        let phase = self.phase.get();
        let done = clock.done(phase);
        match phase {
            Phase::Entering if !done => self.enter.get(),
            Phase::Entering | Phase::Shown => EMPTY_ATOMS,
            Phase::Leaving if !done => self.exit.get(),
            Phase::Removing { .. } => self.exit.get(),
            Phase::Leaving | Phase::Hidden => hidden(self.exit.get()),
        }
    }

    fn resolve(self, node: &Node) {
        let timing = timing(node);
        self.timing.try_set(timing);
        if timing.is_none() && self.phase.try_peek() == Some(Phase::Entering) {
            self.phase.try_set(Phase::Shown);
        }
    }

    fn extend(self, edge: Edge, property: Property) {
        let slot = match edge {
            Edge::Enter => self.enter,
            Edge::Exit => self.exit,
        };
        let added = atoms_of(property, edge.path());
        if added.is_empty() {
            return;
        }
        let mut atoms: SmallVec<[StyleAtom; 16]> = slot.peek().iter().copied().collect();
        atoms.extend(added);
        slot.set(intern_atoms(atoms));
        if edge == Edge::Enter && self.phase.peek() == Phase::Shown {
            self.phase.set(Phase::Entering);
        }
    }
}

pub(super) fn install_motion(node: &mut Node, edge: Edge, property: Property) {
    Presence::of(node).extend(edge, property);
}

pub(super) fn install_presence(node: &mut Node, shown: Signal<bool>) {
    let presence = Presence::of(node);
    presence.extend(Edge::Exit, Property::Opacity(0.0));
    presence.extend(Edge::Enter, Property::Opacity(0.0));
    if !shown.peek() {
        presence.phase.set(Phase::Hidden);
    }
    presence.follow(shown);
}

pub(crate) fn leave(cx: &Cx, id: NodeId) -> bool {
    let Some(node) = cx.get(id) else {
        return false;
    };
    let Some(presence) = node.extension::<Presence>().copied() else {
        return false;
    };
    let timing = timing(node).filter(|_| !MotionPrefs::reduced());
    let exits = presence
        .exit
        .try_peek()
        .is_some_and(|atoms| !atoms.is_empty());
    let shown = presence
        .phase
        .try_peek()
        .is_some_and(|phase| phase != Phase::Hidden);
    if timing.is_none() || !exits || !shown {
        return false;
    }
    presence.timing.try_set(timing);
    presence.phase.try_set(Phase::Removing {
        cx: cx.instance_id(),
        id,
    })
}

pub(crate) fn leaving(node: &Node) -> bool {
    node.extension::<Presence>()
        .and_then(|presence| presence.phase.try_peek())
        .is_some_and(|phase| matches!(phase, Phase::Removing { .. }))
}

pub(crate) fn take_released(cx: u64) -> SmallVec<[NodeId; 4]> {
    RELEASED.with(|cell| {
        let mut mine = SmallVec::new();
        cell.borrow_mut().retain(|&(owner, id)| {
            let ours = owner == cx;
            if ours {
                mine.push(id);
            }
            !ours
        });
        mine
    })
}

fn release(cx: u64, id: NodeId) {
    RELEASED.with(|cell| cell.borrow_mut().push((cx, id)));
    Runtime::mark_frame_dirty();
}

fn timing(node: &Node) -> Option<AnimationConfig> {
    let binding = node.extension::<StyleBinding>()?;
    let atoms: SmallVec<[StyleAtom; 32]> = binding.cascade().collect();
    transition_config(&atoms).map(|(config, _)| config)
}

fn hidden(exit: &'static [StyleAtom]) -> &'static [StyleAtom] {
    intern_atoms(
        exit.iter()
            .copied()
            .chain([StyleAtom::Display(Display::None)])
            .collect(),
    )
}

fn atoms_of(property: Property, path: &'static str) -> SmallVec<[StyleAtom; 16]> {
    let style = StateStyleBuilder::new();
    let style = match property {
        Property::Opacity(value) => style.opacity(value),
        Property::TranslateX(px) => style.translate_x(px),
        Property::TranslateY(px) => style.translate_y(px),
        Property::TranslateZ(px) => style.translate_z(px),
        Property::Scale(factor) => style.scale(factor),
        Property::ScaleX(factor) => style.scale_x(factor),
        Property::ScaleY(factor) => style.scale_y(factor),
        Property::Rotate(radians) => style.rotate(radians.to_degrees()),
        Property::RotateX(radians) => style.rotate_x(radians.to_degrees()),
        Property::RotateY(radians) => style.rotate_y(radians.to_degrees()),
        Property::RotateZ(radians) => style.rotate_z(radians.to_degrees()),
        Property::Background(color) => style.background(color),
        Property::TextColor(color) => style.text_color(color),
        Property::BorderColor(color) => style.border_color(color),
        other => return filter_atoms(style, other, path),
    };
    style.into_atoms()
}

fn filter_atoms(
    style: StateStyleBuilder,
    property: Property,
    path: &'static str,
) -> SmallVec<[StyleAtom; 16]> {
    let style = match property {
        Property::Blur(px) => style.blur(px),
        Property::Brightness(amount) => style.brightness(amount),
        Property::Contrast(amount) => style.contrast(amount),
        Property::Grayscale(amount) => style.grayscale(amount),
        Property::HueRotate(degrees) => style.hue_rotate(degrees),
        Property::Invert(amount) => style.invert(amount),
        Property::Saturate(amount) => style.saturate(amount),
        Property::Sepia(amount) => style.sepia(amount),
        Property::DropShadow(shadow) => style.drop_shadow(shadow),
        Property::TextShadow(shadow) => style.text_shadow(shadow),
        Property::BackdropBlur(px) => style.backdrop_blur(px),
        Property::BackdropBrightness(amount) => style.backdrop_brightness(amount),
        Property::BackdropSaturate(amount) => style.backdrop_saturate(amount),
        other => {
            other.refuse(path);
            return SmallVec::new();
        }
    };
    style.into_atoms()
}
