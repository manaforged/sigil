// Centralised panic helpers for documented contract violations.
// Every panic in this crate routes through here so the lint waiver
// is scoped to a single 30-line module instead of sprinkled across
// 8 call sites. See docs/plans/native-ui/hygiene-cleanup-framework.md.
#![allow(clippy::panic)] // hygiene-exception: hygiene-cleanup-framework.md

use core::panic::Location;

/// Panic helper for "called outside a reactive scope" contract violations.
#[cold]
#[track_caller]
pub(crate) fn no_scope(api: &'static str, caller: &'static Location<'static>) -> ! {
    panic!("{api} called outside a reactive scope (at {caller})")
}

/// Panic helper for "operation on a disposed signal" contract violations.
#[cold]
#[track_caller]
pub(crate) fn disposed_signal(op: &'static str, caller: &'static Location<'static>) -> ! {
    panic!("Signal::{op} on a disposed signal (at {caller})")
}

/// Panic helper for "operation on a disposed entity" contract violations.
///
/// Carries the `EntityGone` token so call sites that catch the panic
/// can pattern-match on the message — mirrors the
/// [`crate::EntityGone`] error returned by the async surface
/// (`AsyncContext::update`) for the same condition.
#[cold]
#[track_caller]
pub(crate) fn disposed_entity(op: &'static str, caller: &'static Location<'static>) -> ! {
    panic!("Entity::{op} on a disposed entity — EntityGone (at {caller})")
}

/// Panic helper for "global of type X not installed" contract violations.
#[cold]
#[track_caller]
pub(crate) fn missing_global(
    op: &'static str,
    type_name: &'static str,
    caller: &'static Location<'static>,
) -> ! {
    panic!("{op}::<{type_name}> not installed in current runtime (at {caller})")
}

/// Diagnostic helper for reactive cycle re-entrancy detection.
#[cold]
#[track_caller]
pub(crate) fn reactive_cycle(loc: Option<&'static Location<'static>>) {
    #[cfg(debug_assertions)]
    panic!("reactive cycle detected: effect at {loc:?} would re-enter itself");

    #[cfg(not(debug_assertions))]
    eprintln!("reactive cycle detected: effect at {loc:?} would re-enter itself; skipping re-run");
}
