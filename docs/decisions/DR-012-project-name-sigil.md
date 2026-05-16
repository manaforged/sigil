# DR-012 - Project name: Sigil

**Status:** Accepted
**Date:** 2026-04-15

## Context

The project began under the working name `sigil`, matching the local
folder and the initial design-plan shorthand. That name described the
technology, not the product. The library now needs a real project name
that can carry documentation, diagnostics, examples, and eventual
distribution without sounding like an ecosystem category.

## Decision

Name the library **Sigil**.

Workspace crates are named `sigil-*` (`sigil-hal`, `sigil-reactive`,
`sigil-kernel`, `sigil-layout`, `sigil-async`, `sigil-renderer`,
`sigil-shell`, and the forthcoming `sigil-text`). The local checkout
may remain in a `sigil` folder until the repository itself is
renamed.

User-facing docs, example window titles, and diagnostic prefixes use
Sigil going forward. Repository URLs and CI badges may remain on the
current hosted `sigil` path until the external repository has actually
been renamed.

### Amendment 2026-04-15: crate rename

This DR originally deferred the crate-name change on the grounds that
it would create "mechanical churn across the workspace." That
rationale was wrong. Every additional `ui-*` crate deepened the
redundancy (the workspace *is* the UI; `ui-hal` reads as "UI's UI
hardware abstraction layer"), and the churn argument only grew with
every new crate. Doing the rename before `sigil-text` lands is
strictly cheaper than doing it after.

The naming convention follows the established 2026 Rust ecosystem
pattern: project-prefix, hyphen at the Cargo-name level, underscore in
`use` paths — the same shape `tokio` / `tokio-util`, `wgpu` /
`wgpu-core` / `wgpu-hal`, and `iced_core` / `iced_widget` use.

## Consequences

Future docs should say Sigil for the library/project. Use `sigil-kernel`,
`sigil-layout`, and the other crate names only when referring to concrete
workspace crates.

The effect trace opt-in environment variable is `SIGIL_TRACE_EFFECTS`,
and emitted lines start with `[sigil trace]`. For a transition window,
the implementation also accepts the legacy `RUST_UI_TRACE_EFFECTS=1`
alias documented by DR-005.

## Kill Criteria

Revisit the name only if the public repository/package strategy needs a
different brand before the first external release.
