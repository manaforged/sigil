# Testing Sigil apps

Sigil apps are testable without a window: the kernel tree, layout
engine, event router, and style resolution all run headless. The
framework's own 1,600+ tests use exactly these surfaces.

## Smoke: the tree builds

```rust,ignore
sigil::testing::with_cx(|cx| {
    let root = app_tree(cx);
    assert!(cx.tree_size(root) > 0);
});
```

`sigil::testing` gives `with_cx` (a scoped `Cx`) and `tree_size` —
enough to prove the tree assembles with no GPU. (A fuller public
interaction-testing API is on the roadmap; today these are the helpers
the examples use.)

## Drive reactivity and events

Everything else composes from public parts:

```rust,ignore
Runtime::scope("my-test", || {
    let mut cx = Cx::new_root();
    let root = cx.add(my_tree());
    let layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
    layout.compute(&mut cx, root, Size::fixed(640.0, 480.0)).unwrap();
    let mut router = EventRouter::new();
    router.route(&mut cx, &layout, pointer_down_event(50.0, 50.0));
    assert_eq!(selected.get(), 1);
});
```

- Synthetic `PointerEvent` / `KeyEvent` route through the same
  `EventRouter` production uses (see `crates/sigil/tests/interactive_state.rs`).
- Layout runs with a stub `TextMeasure` when glyphs don't matter;
  `sigil-text` provides the real engine when they do.
- Paint is assertable offscreen: `sigil_renderer::emit_scene` walks the
  tree into scene commands — check rects, colors, glyph runs (the
  pattern behind the golden suites).

## Golden images

The repo gates on byte-compared goldens: 107 visual tests across
`visual_goldens`, `extra`, `wave2`, `dpi`, and `animated` suites render
widgets through the real wgpu pipeline and diff pixels (channel-tolerant
to absorb AA nondeterminism). Re-bake with `SIGIL_REGEN_GOLDENS=1`;
never rebake blind — see `crates/sigil/tests/golden_support/README.md`.

## Conventions worth copying

- Name tests for the behavior they pin (`hover_overlay_paints_on_pointer_enter`).
- Assert input→pixel, not storage: drive events, read the scene.
- Keep luminance/geometry arithmetic bit-stable across goldens; scope
  float changes to scoped allows.
- Deterministic seeds only — same input, same tree, same pixels.
