//! Debug introspection for the reactive runtime.
//!
//! Exposes dev-time tools promised by the DR-005 addendum:
//!
//! - [`dump_owner_tree`] — human-readable tree of every root owner,
//!   with per-owner signal, effect, cleanup, and child-owner counts.
//! - [`dump_subtree`] — the same renderer scoped to a single owner.
//!
//! These readers walk the thread-local runtime arena. They are
//! purely observational — no reactive state is created, disposed,
//! or mutated. The tools are always available (not gated on
//! `debug_assertions`) because the cost is paid only when a caller
//! actually asks for a dump.
//!
//! Non-goals (explicit for this commit):
//!
//! - No signal-subscriber / effect-dep graph walk (separate tool).
//! - No structured / JSON output (`dump_owner_tree_json` is a later
//!   addition).
//! - No end-of-session leak detector (see the module-level note on
//!   [`run_effect_leak_check`] below).

use core::fmt::Write as _;

use crate::owner::OwnerId;
use crate::runtime::{OwnerKey, OwnerSlot, with_rt};

/// Render the current owner tree as a human-readable string.
///
/// Walks every root owner (no parent) in the thread-local runtime
/// and emits each descendant on its own line. Indentation is two
/// spaces per owner-tree level. Every line shows the owner's debug
/// name and the `file:line` where it was constructed, followed
/// optionally by a bracketed count summary.
///
/// Returns the empty string when the runtime has no live owners.
///
/// # Example output
///
/// ```text
/// App::root  at src/main.rs:14
///   Window(main)  at src/window.rs:42
///     Button(submit)  at src/buttons.rs:10  [signals: 2, effects: 1]
///     For<Account>  at src/list.rs:88  [owners: 3]
///       Row(0)  at src/list.rs:92  [signals: 1, effects: 1]
///       Row(1)  at src/list.rs:92  [signals: 1, effects: 1]
///       Row(2)  at src/list.rs:92  [signals: 1, effects: 1]
/// ```
///
/// See DR-005 addendum item 5.
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

/// Render the subtree rooted at `owner` as a human-readable string.
///
/// The given owner is rendered at depth zero (no indentation); its
/// descendants follow with the same format as [`dump_owner_tree`].
///
/// Returns the empty string if `owner` is not a live owner
/// (already disposed, or from a different runtime).
#[must_use]
pub fn dump_subtree(owner: OwnerId) -> String {
    let mut out = String::new();
    let exists = with_rt(|rt| rt.owners.contains(owner.0));
    if exists {
        write_subtree(&mut out, owner.0, 0);
    }
    out
}

/// Format a single owner slot into its dump line and recurse into
/// children at `depth + 1`.
///
/// Walks the whole subtree under `key`. The runtime borrow is taken
/// once per line to snapshot the slot's fields plus its children
/// list; recursion into children then re-borrows for each child.
/// This pattern matches the rest of the crate — we never hold a
/// runtime borrow across a user-observable boundary.
fn write_subtree(out: &mut String, key: OwnerKey, depth: usize) {
    let Some(rendered) = with_rt(|rt| rt.owners.get(key).map(|slot| render_line(slot, depth)))
    else {
        return;
    };
    out.push_str(&rendered);

    // Snapshot children *after* writing this line so we don't hold
    // the borrow across the recursive call.
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

/// Format one owner slot into the line the dump emits for it.
///
/// Format: `"<indent><name>  at <file>:<line>[  [<counts>]]\n"`.
/// Counts are joined with `", "` inside a single `[...]` block, and
/// only non-zero counts appear. See the module-level example.
fn render_line(slot: &OwnerSlot, depth: usize) -> String {
    let indent = "  ".repeat(depth);
    let name = slot.name;
    let file = slot.location.file();
    let line = slot.location.line();

    // Build the bracketed count summary. Order is fixed so output
    // is stable across runs: signals, effects, cleanups, owners.
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
    // Infallible writes into a String cannot fail; the expect
    // documents that intent without degrading to `unwrap`.
    write!(buf, "{indent}{name}  at {file}:{line}").expect("writing to String is infallible");
    if !parts.is_empty() {
        write!(buf, "  [{}]", parts.join(", ")).expect("writing to String is infallible");
    }
    buf.push('\n');
    buf
}

/// Narrow, correctness-guardrail leak detector for the effect
/// run-owner model.
///
/// Called from [`crate::effect::run_effect`] in `debug_assertions`
/// builds *after* the previous run's effect-owner is disposed.
/// DR-005 §7 of the addendum describes a full end-of-session leak
/// detector; that needs more design (session boundary, Copy-handle
/// drop semantics) and is deferred to a later commit.
///
/// This narrower check verifies the invariant that matters right
/// now: when `run_effect` asks `dispose_owner` to tear down the
/// previous run-owner, the owner slot is actually gone afterwards.
/// A stuck slot means some cleanup path is silently holding a
/// reference, which would accumulate as a slow leak across re-runs
/// of a long-lived effect. We catch it at the next run with an
/// actionable stderr warning rather than shipping a silent leak.
///
/// The check is `debug_assertions`-only; release builds pay nothing.
#[cfg(debug_assertions)]
pub(crate) fn run_effect_leak_check(
    prev_run_owner: OwnerKey,
    effect_name: &'static str,
    effect_location: &'static core::panic::Location<'static>,
) {
    let still_present = with_rt(|rt| rt.owners.contains(prev_run_owner));
    if still_present {
        // Use the `eprintln!` macro rather than `log`/`tracing` so
        // the detector works in tests and release-with-debug builds
        // without any logging infrastructure attached.
        eprintln!(
            "ui-reactive leak: effect run-owner for `{effect_name}` (at {effect_location}) was not disposed before the next run. This is a bug in the effect-owner teardown path."
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::OwnerSlot;

    #[test]
    fn render_line_zero_counts_has_no_bracket() {
        let loc = core::panic::Location::caller();
        let slot = OwnerSlot {
            parent: None,
            children: Vec::new(),
            signals: Vec::new(),
            effects: Vec::new(),
            cleanups: Vec::new(),
            name: "Root",
            location: loc,
        };
        let line = render_line(&slot, 0);
        assert!(line.starts_with("Root  at "));
        assert!(!line.contains('['));
        assert!(line.ends_with('\n'));
    }

    #[test]
    fn render_line_indents_two_spaces_per_depth() {
        let loc = core::panic::Location::caller();
        let slot = OwnerSlot {
            parent: None,
            children: Vec::new(),
            signals: Vec::new(),
            effects: Vec::new(),
            cleanups: Vec::new(),
            name: "Child",
            location: loc,
        };
        let d0 = render_line(&slot, 0);
        let d2 = render_line(&slot, 2);
        assert!(d0.starts_with("Child"));
        assert!(d2.starts_with("    Child"));
    }
}
