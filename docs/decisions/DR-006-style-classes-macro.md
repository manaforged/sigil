# DR-006 — Style: `classes!` macro plus struct API split

**Status:** Accepted
**Date:** 2026-04-14

## Context

We want the shadcn + Tailwind design vocabulary — utility classes,
variants, theme tokens — adapted to native Rust without inheriting the
web's impedance mismatches.

Research on compile-time utility class resolution (April 2026):

- **UnoCSS** scans templates at build time and synthesises utilities to
  CSS on demand. The "compilation mode" can merge multiple classes into
  one at build. Proves the pattern works at scale in JS tooling.
- **vanilla-extract** evaluates `.css.ts` files at build, emits static
  CSS, nothing ships to runtime. Closest analogue to "write styles in
  code, get atoms out."
- **Panda CSS** ships a runtime resolver — the counter-example: refusing
  the build step costs runtime performance.
- **StyleX** (Meta) does static extraction plus atomic CSS with
  deterministic specificity ordering.
- **No prior art in Rust.** A proc macro can take a string literal,
  parse utility syntax, look up atoms in a `phf` static table, and emit
  a `&[StyleAtom]` array.

Risk findings:

- rust-analyzer macro expansion is measurably slower than rustc's. Heavy
  `classes!` use will show up as LSP latency before it shows up as
  compile time.
- Proc macros have no program-wide context — they cannot validate that
  a theme token exists without a separate registration step.
- Macro-generated code is a debugging black hole. Leptos `view!` has
  exactly this complaint in community feedback.
- The previous draft conflated "compile-time utility resolution" with
  "reactive style binding." These are different systems.

## Decision

Split the style API into three layers:

### 1. `classes!` macro — static compile-time utilities

Takes string literals and `match`-bound string literals, produces a
`&'static [StyleAtom]`. Backed by a `phf` static atom table and a small
hand-written parser.

```rust
let base = classes!(
    "inline-flex items-center justify-center rounded-md",
    match self.size {
        Size::Sm => "h-8 px-3 text-sm",
        Size::Md => "h-10 px-4 text-base",
    },
);
```

Rules:

- No string interpolation. No `bg-${color}-500`. The macro refuses.
- Theme tokens (`bg-accent`) resolve at runtime through the active `Theme`.
- Target: <5ms per invocation, <500 LoC of macro implementation.
- Hard rule: if `classes!` becomes the slowest thing in compile times,
  we delete it and fall back to the struct API.
- rust-analyzer macro expansion latency is a tracked budget from
  Phase 4 onward.

### Compile-time vs runtime token validation

Proc macros have no program-wide context. They cannot see an
app-level theme struct at expansion time, and expansion order is not
a reliable cross-crate registry. An earlier draft of this DR proposed
a `register_theme_tokens!` declarative step populating a shared `phf`
map at compile time; that idea is rejected here because it would
either reject legitimate downstream theme tokens (component crate
compiles before the consuming app registers its theme) or silently
force theme tokens through a runtime path the macro cannot see.

The actual model has two token tiers:

1. **Structural utilities** — layout, spacing, sizing, typography
   scale, border radius, and opacity classes. These come from a
   fixed grammar defined in `ui-style` and baked into `classes!` via
   a `phf` table compiled *into the macro crate itself*. No
   registration step, no cross-crate coordination. `classes!`
   validates these at compile time and emits a hard error on unknown
   structural tokens. Example: `classes!("px-4 rounded-md h-10")`
   all resolve inside the macro crate.

2. **Theme tokens** — colour names, semantic roles, shadow presets,
   motion durations. These vary by app and are defined in a `Theme`
   struct at runtime. `classes!` recognises theme tokens by prefix
   (`bg-*`, `text-*`, `border-*`, `shadow-*`, `motion-*`) and emits
   a `StyleAtom::ThemeRef(&'static str)` for each, deferring
   validation to the first `Theme` binding at app construction time.
   Unknown theme tokens panic in dev builds on first resolution with
   a message listing the registered tokens and suggesting the
   nearest matches; in release they render as the theme's
   `missing_token_fallback` (typically a debug magenta).

Apps that want compile-time validation of their own theme tokens
generate a small `theme_tokens.rs` from their theme definition via a
`build.rs` in the *app crate* (or a separate `cargo-sigil gen-tokens`
command as a convenience). The generated file is an explicit input
to the app, not a magic cross-crate macro registry. Library crates
that build components never see app-specific theme tokens at compile
time — they emit `ThemeRef` atoms and let the app's theme resolve
them at runtime.

This is the explicit source-of-truth model the compile-time-only
attempt could not provide, without pretending proc macros have
program-wide state.

### 2. `Style::new()` struct API — always available

```rust
let style = Style::new().padding(8).bg(theme.accent);
```

Produces the same `StyleAtom` array. The struct API is the always-available
escape hatch for developers who dislike macro-generated code or who hit
the debugging black hole. **Not secondary — first-class peer.**

### 3. `.style_reactive(|| ...)` — signal-driven styles

```rust
cx.node().style_reactive(move || {
    if hovered.get() { classes!("bg-accent-hover") } else { &[] }
})
```

Creates a reactive effect that re-applies the returned atoms when the
signal dependencies change. This is a **separate system** from
`classes!` — static resolution vs reactive binding are not the same
problem.

## Consequences

- Every component ships the same `StyleAtom` array regardless of which
  API built it. One render path, one atom type, one merge implementation.
- Consumer-passed classes always win over component defaults. The
  `merge_classes(default, override)` helper detects conflicting atoms
  (e.g. two `padding-x` values) and resolves last-wins with a dev-build
  warning.
- The dual API doubles the documentation surface. Every primitive's
  docs show both forms. This is the cost of making customization
  genuinely easy.

## Kill criteria

Delete `classes!` and keep only the struct API if any of:

1. Compile time budget blown: `classes!` invocations dominate `cargo
   build` time beyond a measured <5ms per call.
2. rust-analyzer macro expansion latency makes LSP unusable in large
   component files.
3. Debugging experience around macro-expanded code is cited as a major
   pain in downstream feedback during Phase 6 or later.

The struct API is designed to be sufficient on its own. The macro is
the ergonomic layer, not the foundation.

## References

- [UnoCSS](https://unocss.dev/guide/)
- [vanilla-extract](https://vanilla-extract.style/)
- [Panda CSS](https://panda-css.com/)
- [phf crate](https://github.com/rust-phf/rust-phf)
