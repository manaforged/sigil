//! Edge-triggered event channels and the state/edge trigger wrapper.

use core::marker::PhantomData;

use crate::effect::effect;
use crate::entity::Entity;
use crate::entity_observer::Subscription;
use crate::owner::Owner;
use crate::signal::Signal;

/// An edge-triggered one-shot broadcast channel.
///
/// Emitting a value notifies all current subscribers once; no `T`
/// value is retained between emits. When to use: choose
/// `EventChannel<T>` for edge-triggered fire-once events such as
/// dismissing a toast or playing a sound, and choose [`Signal<T>`]
/// for level-triggered state with a meaningful current value.
#[derive(Clone, Copy)]
pub struct EventChannel<T: 'static> {
    entity: Entity<()>,
    _value: PhantomData<fn() -> T>,
}

impl<T: Clone + 'static> EventChannel<T> {
    /// Create an empty edge-triggered channel.
    ///
    /// When to use: create an `EventChannel<T>` when future emits are
    /// meaningful but there is no current state to read; use
    /// [`Signal<T>`] instead when callers need level-triggered state.
    ///
    /// # Panics
    ///
    /// Panics if called outside a reactive scope.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entity: Entity::new("EventChannel", |_cx| ()),
            _value: PhantomData,
        }
    }

    /// Broadcast `value` to all current subscribers without retaining it.
    ///
    /// When to use: call `emit` for edge-triggered fire-once events;
    /// use [`Signal::set`] for level-triggered state changes that
    /// should leave a readable current value behind.
    pub fn emit(&self, value: T) {
        self.entity.update(|(), cx| cx.emit(value));
    }

    /// Register a subscriber for future emits.
    ///
    /// Dropping the returned [`Subscription`] deregisters the
    /// callback. When to use: subscribe to an `EventChannel<T>` for
    /// edge-triggered events that fire only when emitted; subscribe to
    /// a [`Signal<T>`] or [`Trigger::State`] when a persistent current
    /// value is part of the contract.
    ///
    /// # Panics
    ///
    /// Panics if called outside a reactive scope.
    #[must_use = "dropping the subscription immediately unregisters the callback"]
    pub fn subscribe(&self, f: impl Fn(T) + 'static) -> Subscription {
        let source = self.entity;
        let mut subscription = None;
        let _subscriber = Entity::new("EventChannel.subscriber", |cx| {
            subscription = Some(
                cx.subscribe::<(), T>(&source, move |(), _entity, value, _cx| {
                    f(value.clone());
                }),
            );
        });
        subscription.expect("EventChannel subscriber registration did not produce a subscription")
    }
}

impl<T: Clone + 'static> Default for EventChannel<T> {
    fn default() -> Self {
        Self::new()
    }
}
/// A typed reactive trigger that distinguishes state from events.
///
/// When to use: choose [`Trigger::State`] for level-triggered,
/// state-driven surfaces with a readable current value, and choose
/// [`Trigger::Edge`] for edge-triggered fire-once surfaces with no
/// retained state.
pub enum Trigger<T: 'static> {
    /// Level-triggered: wraps an existing [`Signal<T>`].
    ///
    /// When to use: choose `State` when the reactive value has a
    /// persistent current state, such as whether a modal is open.
    State(Signal<T>),
    /// Edge-triggered: wraps an [`EventChannel<T>`].
    ///
    /// When to use: choose `Edge` for fire-once imperative events
    /// that have no meaningful current state, such as dismissing a
    /// toast or playing a sound.
    Edge(EventChannel<T>),
}

impl<T: Clone + 'static> Trigger<T> {
    /// Build a level-triggered trigger from a signal.
    ///
    /// When to use: call `from_signal` when an existing
    /// [`Signal<T>`] is the source of truth and callers should still
    /// treat the surface as level-triggered rather than edge-triggered.
    #[must_use]
    pub const fn from_signal(signal: Signal<T>) -> Self {
        Self::State(signal)
    }

    /// Return the current value for a level-triggered trigger.
    ///
    /// When to use: call `current` only for state-driven triggers
    /// that retain a value; edge-triggered triggers intentionally have
    /// no current state and this method panics for them.
    ///
    /// # Panics
    ///
    /// Panics if called on [`Trigger::Edge`].
    #[must_use]
    #[expect(
        clippy::panic,
        reason = "edge triggers intentionally panic when read as state"
    )]
    pub fn current(&self) -> T
    where
        T: Clone,
    {
        match self {
            Self::State(signal) => signal.peek(),
            Self::Edge(_) => panic!("Trigger::current called on an edge trigger with no state"),
        }
    }

    /// Subscribe to future trigger values.
    ///
    /// For [`Trigger::State`], the callback fires when the wrapped
    /// signal changes. For [`Trigger::Edge`], it fires on each emit.
    /// When to use: call `subscribe` when code can handle both
    /// level-triggered signal changes and edge-triggered fire-once
    /// events through a single typed surface.
    ///
    /// # Panics
    ///
    /// Panics if called outside a reactive scope.
    #[must_use = "dropping the subscription immediately unregisters the callback"]
    pub fn subscribe(&self, f: impl Fn(T) + 'static) -> Subscription {
        match self {
            Self::State(signal) => subscribe_signal(*signal, f),
            Self::Edge(channel) => channel.subscribe(f),
        }
    }
}

fn subscribe_signal<T: Clone + 'static>(
    signal: Signal<T>,
    f: impl Fn(T) + 'static,
) -> Subscription {
    let parent = Owner::current();
    let owner = Owner::new("Trigger::State.subscribe", parent);
    owner.with(|| {
        let mut initialized = false;
        effect(move || {
            let value = signal.get();
            if initialized {
                f(value);
            } else {
                initialized = true;
            }
        });
    });
    Subscription::cleanup(move || owner.dispose())
}
