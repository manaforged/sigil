use core::fmt::Write as _;

use crate::entity::{Entity, EntityId};
use crate::owner::OwnerId;
use crate::runtime::{OwnerKey, OwnerSlot, with_rt};

#[must_use]
pub fn dump_owner_tree() -> String {
    let roots = with_rt(|rt| {
        rt.owners
            .keys()
            .filter(|&key| rt.owners.get(key).is_some_and(|slot| slot.parent.is_none()))
            .collect::<Vec<_>>()
    });
    let mut out = String::new();
    for root in roots {
        write_subtree(&mut out, root, 0);
    }
    out
}

#[doc(hidden)]
#[must_use]
pub fn entity_is_live<T: 'static>(entity: Entity<T>) -> bool {
    with_rt(|rt| rt.entities.contains(entity.key))
}

#[doc(hidden)]
#[must_use]
pub fn entity_id_is_live(id: EntityId) -> bool {
    with_rt(|rt| rt.entities.contains(id.0))
}

#[must_use]
pub fn dump_subtree(owner: OwnerId) -> String {
    let mut out = String::new();
    let exists = with_rt(|rt| rt.owners.contains(owner.0));
    if exists {
        write_subtree(&mut out, owner.0, 0);
    }
    out
}

fn write_subtree(out: &mut String, key: OwnerKey, depth: usize) {
    let Some(rendered) = with_rt(|rt| rt.owners.get(key).map(|slot| render_line(slot, depth)))
    else {
        return;
    };
    out.push_str(&rendered);

    let children = with_rt(|rt| {
        rt.owners
            .get(key)
            .map(|slot| slot.children.clone())
            .unwrap_or_default()
    });
    for child in children {
        write_subtree(out, child, depth + 1);
    }
}

fn render_line(slot: &OwnerSlot, depth: usize) -> String {
    let indent = "  ".repeat(depth);
    let name = slot.name;
    let file = slot.location.file();
    let line = slot.location.line();

    let mut parts: Vec<String> = Vec::new();
    let signals = slot.signals.len();
    let effects = slot.effects.len();
    let cleanups = slot.cleanups.len();
    let owners = slot.children.len();
    if signals > 0 {
        parts.push(format!("signals: {signals}"));
    }
    if effects > 0 {
        parts.push(format!("effects: {effects}"));
    }
    if cleanups > 0 {
        parts.push(format!("cleanups: {cleanups}"));
    }
    if owners > 0 {
        parts.push(format!("owners: {owners}"));
    }

    let mut buf = String::new();
    write!(buf, "{indent}{name}  at {file}:{line}").expect("writing to String is infallible");
    if !parts.is_empty() {
        write!(buf, "  [{}]", parts.join(", ")).expect("writing to String is infallible");
    }
    buf.push('\n');
    buf
}

#[cfg(debug_assertions)]
pub(crate) fn run_effect_leak_check(
    prev_run_owner: OwnerKey,
    effect_name: &'static str,
    effect_location: &'static core::panic::Location<'static>,
) {
    let still_present = with_rt(|rt| rt.owners.contains(prev_run_owner));
    if still_present {
        eprintln!(
            "sigil-reactive leak: effect run-owner for `{effect_name}` (at {effect_location}) was not disposed before the next run. This is a bug in the effect-owner teardown path."
        );
    }
}

#[cfg(test)]
#[path = "debug_tests.rs"]
mod tests;
