use std::fmt::Write as _;

use crate::reactive::owner::OwnerId;
use crate::reactive::runtime::{OwnerKey, OwnerSlot, with_rt};

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

#[must_use]
pub fn dump_subtree(owner: OwnerId) -> String {
    let mut out = String::new();
    write_subtree(&mut out, owner.0, 0);
    out
}

fn write_subtree(out: &mut String, key: OwnerKey, depth: usize) {
    let Some(children) = with_rt(|rt| {
        rt.owners.get(key).map(|slot| {
            out.push_str(&render_line(slot, depth));
            slot.children.clone()
        })
    }) else {
        return;
    };
    for child in children {
        write_subtree(out, child, depth + 1);
    }
}

fn render_line(slot: &OwnerSlot, depth: usize) -> String {
    let counts = [
        ("signals", slot.signals.len()),
        ("effects", slot.effects.len()),
        ("cleanups", slot.cleanups.len()),
        ("owners", slot.children.len()),
    ];
    let parts: Vec<String> = counts
        .iter()
        .filter(|(_, count)| *count > 0)
        .map(|(label, count)| format!("{label}: {count}"))
        .collect();
    let mut line = format!(
        "{}{}  at {}:{}",
        "  ".repeat(depth),
        slot.name,
        slot.location.file(),
        slot.location.line()
    );
    if !parts.is_empty() {
        let _ = write!(line, "  [{}]", parts.join(", "));
    }
    line.push('\n');
    line
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
