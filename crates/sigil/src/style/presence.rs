use std::cell::RefCell;
use std::rc::Rc;

use crate::animation_runtime::{AnimationConfig, Animator};
use crate::kernel::Node;
use crate::reactive::{Signal, batch, effect_named, signal};

use super::install::{install_animation, install_reactive};
use super::{DisplayKind, NumericAtomKind, StyleAtom};

pub(crate) struct Presence {
    shown: Signal<bool>,
}

impl Presence {
    pub(crate) fn interactive(&self) -> bool {
        self.shown.peek()
    }
}

pub(super) fn install_presence(node: &mut Node, shown: Signal<bool>, config: AnimationConfig) {
    let opacity = signal(0.0);
    let animator = Rc::new(RefCell::new(Animator::new(opacity, config)));
    let target = Rc::clone(&animator);
    effect_named("sigil-style::presence", move || {
        batch(|| {
            target
                .borrow_mut()
                .target(if shown.get() { 10_000.0 } else { 0.0 });
        });
    });
    install_animation(node, NumericAtomKind::Opacity, opacity);
    install_reactive(node, move || {
        let _value = opacity.get();
        if shown.get() || !animator.borrow().settled() {
            &[]
        } else {
            &[StyleAtom::Display(DisplayKind::None)]
        }
    });
    let _previous = node.attach(Presence { shown });
}
