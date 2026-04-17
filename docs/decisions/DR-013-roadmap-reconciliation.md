# DR-013 - Roadmap reconciliation and plan traceability

**Status:** Accepted
**Date:** 2026-04-15

## Context

The original master roadmap was written before the repository's actual
implementation sequence settled. It described a large kernel slice and
assigned later phase numbers to layout/text/style work. The codebase now
has a different, more incremental sequence:

- foundation crates and the first runnable proof,
- `sigil-core` as the shared node model,
- `sigil-layout` as the first downstream adapter.

DR-010 and DR-011 accurately describe what shipped, but they previously
depended on local user-home execution plans. That made the record
unverifiable for anyone without the same machine.

## Decision

The decision records and repo-local plans are the canonical source of
truth for shipped milestones. The older master roadmap remains useful
strategy background, but its phase numbers are superseded for this
repository.

Execution plans that a DR depends on must live in the repo under
`docs/plans/` before the DR is considered complete. DRs may reference
external research notes, but they must not depend on a Windows-absolute
user path for the core argument.

For current shipped work:

- [`phase-03-sigil-core-keystone.md`](../plans/phase-03-sigil-core-keystone.md)
  is the repo-local `sigil-core` plan.
- [`phase-04-sigil-layout-adapter.md`](../plans/phase-04-sigil-layout-adapter.md)
  is the repo-local `sigil-layout` plan.

Public README status should name the shipped capability rather than rely
on phase numbers whose meaning has changed over time.

## Consequences

Future milestones should be identified by crate/capability first and by
phase number second, if at all. A phase label is not authoritative unless
the matching repo-local plan and DR agree.

The following follow-ups are tracked here until they receive their own
DR or plan:

- **F-001 Fragment layout semantics:** **Closed 2026-04-16** by
  [DR-015](DR-015-fragment-semantics.md) — flatten into parent, with
  `debug_assert!` footguns on `class`/`theme`/`style_reactive` /
  `focusable(true)` against fragments.
- **F-002 sigil-shell async handle:** replace the temporary shell callback
  hook with a typed `sigil_async::AsyncRuntime` handle once that API is
  stable enough for the shell boundary.
- **F-003 Layout result projection:** keep `LayoutResult` immutable to
  downstream consumers. If a future crate needs to mutate geometry,
  introduce a new typed extension rather than mutating layout output.

## Kill Criteria

Revisit this decision if the project publishes a new master roadmap in
the repo that replaces these milestone plans and explicitly maps old
phase numbers to new ones.

## References

- [DR-010 - sigil-core keystone crate](./DR-010-sigil-core-keystone.md)
- [DR-011 - sigil-layout adapter](./DR-011-sigil-layout-adapter.md)
