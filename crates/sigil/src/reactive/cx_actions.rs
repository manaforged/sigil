use core::any::Any;
use core::panic::Location;

use crate::actions::{Action, ActionRegistry, HandlerKey, Propagate};
use smallvec::SmallVec;

use crate::reactive::cx::EntityCx;
use crate::reactive::focus::FocusHandle;
use crate::reactive::runtime::with_rt;

impl<T: 'static> EntityCx<'_, T> {
    pub fn action<A: Action>(
        &mut self,
        handle: &FocusHandle,
        handler: impl FnMut(&A, &mut Propagate) + 'static,
    ) {
        let key = HandlerKey::from(handle.id());
        with_rt(|rt| {
            let registry = rt.action_registry.get_or_insert_with(ActionRegistry::new);
            registry.register::<A>(key, handler);
        });
    }

    #[track_caller]
    pub fn dispatch<A: Action>(&mut self, action: &A) {
        let painting = with_rt(|rt| rt.phase == crate::reactive::DrawPhase::Paint);
        if painting {
            with_rt(|rt| crate::reactive::phase::assert_paint_safe(rt, "EntityCx::dispatch"));
            let caller = Location::caller();
            let cloned: Box<dyn Action> = action.boxed_clone();
            let replay: crate::reactive::phase::ReplayFn = Box::new(move || {
                dispatch_replay_action(cloned.as_ref());
            });
            crate::reactive::phase::enqueue_deferred(replay, caller);
            return;
        }
        let chain: SmallVec<[HandlerKey; 8]> = with_rt(|rt| {
            rt.focus_stack
                .iter()
                .copied()
                .map(HandlerKey::from)
                .collect()
        });
        let Some(mut taken) = with_rt(|rt| {
            rt.action_registry
                .as_mut()
                .map(ActionRegistry::take_entries::<A>)
        }) else {
            return;
        };
        if taken.is_empty() {
            with_rt(|rt| {
                if let Some(registry) = rt.action_registry.as_mut() {
                    registry.restore_entries(taken);
                }
            });
            return;
        }
        let _result = ActionRegistry::dispatch_taken(&mut taken, action, &chain);
        with_rt(|rt| {
            if let Some(registry) = rt.action_registry.as_mut() {
                registry.restore_entries(taken);
            }
        });
    }
}

fn dispatch_replay_action(action: &dyn Action) {
    let chain: SmallVec<[HandlerKey; 8]> = with_rt(|rt| {
        rt.focus_stack
            .iter()
            .copied()
            .map(HandlerKey::from)
            .collect()
    });
    let type_id = Any::type_id(action.as_any());
    let Some(mut taken) = with_rt(|rt| {
        rt.action_registry
            .as_mut()
            .map(|registry| registry.take_entries_by_id(type_id))
    }) else {
        return;
    };
    if taken.is_empty() {
        with_rt(|rt| {
            if let Some(registry) = rt.action_registry.as_mut() {
                registry.restore_entries(taken);
            }
        });
        return;
    }
    let _ = ActionRegistry::dispatch_taken_dyn(&mut taken, action, &chain);
    with_rt(|rt| {
        if let Some(registry) = rt.action_registry.as_mut() {
            registry.restore_entries(taken);
        }
    });
}
