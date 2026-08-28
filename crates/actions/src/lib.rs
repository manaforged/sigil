use std::any::{Any, TypeId};
use std::collections::HashMap;

use smallvec::SmallVec;

#[cfg(test)]
use criterion as _;
#[cfg(test)]
use sigil as _;

pub use sigil_actions_macros::Action as DeriveAction;

pub trait Action: Any + Send + 'static {
    fn debug_name(&self) -> &'static str;

    fn boxed_clone(&self) -> Box<dyn Action>;

    fn as_any(&self) -> &dyn Any;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct HandlerKey(u64);

impl HandlerKey {
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    #[must_use]
    pub const fn as_raw(self) -> u64 {
        self.0
    }
}

pub struct Propagate {
    propagate: bool,
}

impl Propagate {
    const fn new() -> Self {
        Self { propagate: false }
    }

    pub const fn propagate(&mut self) {
        self.propagate = true;
    }

    pub(crate) const fn should_propagate(&self) -> bool {
        self.propagate
    }
}

#[derive(Debug, Clone)]
pub struct DispatchResult {
    handled: bool,
    fired_owners: SmallVec<[HandlerKey; 4]>,
}

impl DispatchResult {
    #[must_use]
    pub const fn handled(&self) -> bool {
        self.handled
    }

    #[must_use]
    pub fn fired_owners(&self) -> &[HandlerKey] {
        &self.fired_owners
    }
}

type Handler = Box<dyn FnMut(&dyn Any, &mut Propagate)>;

struct HandlerEntry {
    owner: HandlerKey,
    handler: Handler,
}

#[derive(Default)]
pub struct ActionRegistry {
    handlers: HashMap<TypeId, SmallVec<[HandlerEntry; 4]>>,
}

impl ActionRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<A: Action>(
        &mut self,
        owner: HandlerKey,
        mut handler: impl FnMut(&A, &mut Propagate) + 'static,
    ) {
        let entry = HandlerEntry {
            owner,
            handler: Box::new(move |any, cx| {
                if let Some(action) = any.downcast_ref::<A>() {
                    handler(action, cx);
                }
            }),
        };
        self.handlers
            .entry(TypeId::of::<A>())
            .or_default()
            .push(entry);
    }

    pub fn dispatch<A: Action>(&mut self, action: &A, chain: &[HandlerKey]) -> DispatchResult {
        let Some(entries) = self.handlers.get_mut(&TypeId::of::<A>()) else {
            return DispatchResult {
                handled: false,
                fired_owners: SmallVec::new(),
            };
        };
        Self::dispatch_entries(entries, action as &dyn Any, chain)
    }

    pub fn dispatch_dyn(&mut self, action: &dyn Action, chain: &[HandlerKey]) -> DispatchResult {
        let type_id = Any::type_id(action.as_any());
        let Some(entries) = self.handlers.get_mut(&type_id) else {
            return DispatchResult {
                handled: false,
                fired_owners: SmallVec::new(),
            };
        };
        Self::dispatch_entries(entries, action.as_any(), chain)
    }

    fn dispatch_entries(
        entries: &mut SmallVec<[HandlerEntry; 4]>,
        action: &dyn Any,
        chain: &[HandlerKey],
    ) -> DispatchResult {
        let mut fired_owners: SmallVec<[HandlerKey; 4]> = SmallVec::new();
        for owner in chain.iter().rev() {
            let mut any_at_owner = false;
            let mut should_continue = false;
            for entry in entries.iter_mut().filter(|e| e.owner == *owner) {
                let mut cx = Propagate::new();
                (entry.handler)(action, &mut cx);
                any_at_owner = true;
                if cx.should_propagate() {
                    should_continue = true;
                }
            }
            if any_at_owner {
                fired_owners.push(*owner);
                if !should_continue {
                    return DispatchResult {
                        handled: true,
                        fired_owners,
                    };
                }
            }
        }
        DispatchResult {
            handled: !fired_owners.is_empty(),
            fired_owners,
        }
    }

    pub fn take_entries<A: Action>(&mut self) -> TakenEntries {
        self.take_entries_by_id(TypeId::of::<A>())
    }

    pub fn take_entries_by_id(&mut self, type_id: TypeId) -> TakenEntries {
        let entries = self.handlers.remove(&type_id).unwrap_or_default();
        TakenEntries { type_id, entries }
    }

    pub fn restore_entries(&mut self, taken: TakenEntries) {
        let slot = self.handlers.entry(taken.type_id).or_default();
        slot.extend(taken.entries);
    }

    pub fn dispatch_taken<A: Action>(
        taken: &mut TakenEntries,
        action: &A,
        chain: &[HandlerKey],
    ) -> DispatchResult {
        debug_assert_eq!(
            taken.type_id,
            TypeId::of::<A>(),
            "dispatch_taken type mismatch: entries were taken for a different action type",
        );
        Self::dispatch_entries(&mut taken.entries, action as &dyn Any, chain)
    }

    pub fn dispatch_taken_dyn(
        taken: &mut TakenEntries,
        action: &dyn Action,
        chain: &[HandlerKey],
    ) -> DispatchResult {
        debug_assert_eq!(
            taken.type_id,
            Any::type_id(action.as_any()),
            "dispatch_taken_dyn type mismatch: entries were taken for a different action type",
        );
        Self::dispatch_entries(&mut taken.entries, action.as_any(), chain)
    }
}

#[must_use = "TakenEntries must be restored via ActionRegistry::restore_entries"]
pub struct TakenEntries {
    type_id: TypeId,
    entries: SmallVec<[HandlerEntry; 4]>,
}

impl TakenEntries {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
