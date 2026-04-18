# Architecture Decision Records

Every load-bearing architectural choice in this project gets a decision
record. The goal is not ceremony; it is so that future contributors and
future us can see why a choice was made and when it should be revisited.

Each record has four sections:

- **Context** - the problem and the evidence we considered
- **Decision** - what we chose
- **Consequences** - what this commits us to
- **Kill criteria** - what would make us change our mind

Records are immutable once merged. Superseding a record means adding a
new DR that references the old one, not editing the old one.

## Index

| ID | Title | Status |
| --- | --- | --- |
| 000 | Why not fork Floem | Accepted |
| 001 | Renderer - wgpu upstream, HAL-wrapped | Accepted |
| 002 | Shell - winit 0.30+ with async bridge | Accepted |
| 003 | Layout - Taffy upstream behind HAL, no fork day one | Accepted |
| 004 | Text - Parley with owned IME/editor completion | Accepted |
| 005 | Reactivity - Leptos 0.5+ owner tree | Accepted |
| 006 | Style - `classes!` macro + struct API split | Accepted |
| 007 | Async - one Tokio runtime per app | Accepted |
| 008 | Accessibility - AccessKit TreeUpdate from Phase 1 | Accepted |
| 009 | Performance discipline - bench tiers, budgets, CI regression gate | Accepted |
| 010 | sigil-core - keystone crate | Accepted |
| 011 | sigil-layout - Taffy-backed adapter | Accepted |
| 012 | Project name: Sigil | Accepted |
| 013 | Roadmap reconciliation and plan traceability | Accepted |
| 014 | Text Renderer: Slug GPU Algorithm | Accepted |
| 015 | Fragment semantics: flatten into parent | Accepted |
| 016 | Application shell: `sigil::launch` | Accepted |
| 017 | Default scene paint in `sigil-renderer` | Accepted (text-deferral superseded by 025) |
| 018 | DPI and logical-pixel unit system | Accepted |
| 020 | Scene command buffer | Accepted |
| 023 | Incremental layout — generation gating | Accepted (cache-key superseded by 024) |
| 024 | Layout cache-key content hash | Accepted |
| 025 | Text-in-scene interim wiring | Accepted |
| 026 | Widget substrate contract | Accepted |
| 027 | Text input contract | Accepted |
