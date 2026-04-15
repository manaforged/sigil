# Architecture Decision Records

Every load-bearing architectural choice in this project gets a decision
record. The goal is not ceremony — it is so that future contributors (and
future us) can see *why* a choice was made and *when* it should be
revisited.

Each record has four sections:

- **Context** — the problem and the evidence we considered
- **Decision** — what we chose
- **Consequences** — what this commits us to
- **Kill criteria** — what would make us change our mind

Records are immutable once merged. Superseding a record means adding a
new DR that references the old one, not editing the old one.

## Index

| ID  | Title                                              | Status   |
| --- | -------------------------------------------------- | -------- |
| 000 | Why not fork Floem                                 | Accepted |
| 001 | Renderer — wgpu upstream, HAL-wrapped              | Accepted |
| 002 | Shell — winit 0.30+ with async bridge              | Accepted |
| 003 | Layout — Taffy upstream behind HAL, no fork day one| Accepted |
| 004 | Text — Parley with owned IME/editor completion     | Accepted |
| 005 | Reactivity — Leptos 0.5+ owner tree                | Accepted |
| 006 | Style — `classes!` macro + struct API split        | Accepted |
| 007 | Async — one Tokio runtime per app                  | Accepted |
| 008 | Accessibility — AccessKit TreeUpdate from Phase 1  | Accepted |
| 009 | Performance discipline — bench tiers, budgets, CI regression gate | Accepted |
