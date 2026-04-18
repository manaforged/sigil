# DR-026 - Widget substrate contract

**Status:** Accepted
**Date:** 2026-04-17
**Depends on:** DR-006 (style API), DR-024 (layout cache-key content hash), DR-025 (text-in-scene interim wiring).

## Context

Commit `9b9f851` introduced the first shared widget substrate:
`InteractionState`, `StyleState`, `StyleVariant`, `StateStyle`, and
`FocusRing`. At that point only `button` consumed the contract, so the
surface had not yet been proven across adjacent widget shapes.

Widget Pack 1 added three more controls to pressure-test the contract:

- `checkbox` - binary state with checked/unchecked visual variants.
- `toggle` - binary state with a distinct track/thumb visual language.
- `slider` - pointer capture plus drag-to-value behavior.

The slider also exposed a coordinate-space gap: widget handlers need
target-local pointer coordinates for nested controls, while hit-testing
must continue to use window-relative coordinates.

## Decision

`InteractionState` is the contract every interactive built-in widget
uses for hover, press, focus, disabled styling, and activation.

### Shipped contract

1. Widgets construct `InteractionState` inside their reactive owner and
   install it on the widget root.
2. Widgets derive visual state through `InteractionState::style_state()`
   and compose it with `StyleVariant`.
3. Widgets use `InteractionState::release_pointer()` and
   `InteractionState::activate_key()` for activation paths instead of
   open-coding pressed/focused logic.
4. Widgets attach `FocusRing` through `InteractionState::install()`.
5. Widget roots carry the relevant accessibility role and reactive
   state:
   - `Role::Button` for `button`
   - `Role::CheckBox` plus `A11yProps::checked` for `checkbox`
   - `Role::Switch` plus `A11yProps::checked` for `toggle`
   - `Role::Slider` plus `A11yProps::value_now`, `value_min`, and
     `value_max` for `slider`

### Step 4 delta

Slider needed a drag state that is distinct from "pressed" but should
render as pressed while a pointer capture drag is active.

The accepted `InteractionState` delta is:

- `pub fn dragging(self) -> bool`
- `pub fn set_dragging(self, next: bool)`

Internally, `InteractionState` stores `dragging: Signal<bool>`.
`style_state()` treats `pressed || dragging` as the visual pressed state,
while `dragging()` remains separately queryable for drag routing logic.

### Pointer coordinate contract

The event router still receives shell pointer events in window-relative
logical pixels for hit-testing. When invoking a pointer handler, it
temporarily presents `PointerEvent::position` in the current target
node's local coordinate space. Captured pointer moves are localized to
the captured target.

This keeps widget math local: `slider` can map `event.position.x` onto
its track width whether it is mounted at the root or nested inside the
vertical-slice controls panel.

### Future widget rule

Every future interactive widget must use:

- `InteractionState`
- `StyleVariant` / `StateStyle`
- `FocusRing`

A widget that cannot use that stack must justify the deviation in its
own decision record.

## Consequences

- Built-in widgets now share one interaction path instead of each
  maintaining local hover/press/focus bookkeeping.
- `button`, `checkbox`, `toggle`, and `slider` are the reference
  examples for future widget authors.
- `sigil-core` accessibility props now include reactive checked and
  numeric range values so semantic state can live beside the visual
  widget state.
- `sigil-events` pointer dispatch now has a clearer split:
  window-relative coordinates for routing, target-local coordinates for
  handlers.
- The vertical-slice example renders all four widgets and exercises
  nested slider dragging through the normal event router.

## Alternatives Rejected

| Alternative | Reason |
|---|---|
| Keep drag state slider-local | Fails the point of the substrate pass; visual pressed state would be duplicated outside `InteractionState`. |
| Add a larger drag API with start position and deltas | The slider only needed a queryable active-drag bit. More state would have been speculative. |
| Make slider compensate for global pointer coordinates itself | Widgets do not know their absolute layout origin. Localizing in the router fixes all nested pointer widgets. |
| Let each binary widget define its own state-style helpers | Produces parallel hover/press/disabled logic and weakens the contract being tested. |

## Kill Criteria

Revisit this DR if:

- A future widget needs more than a small `InteractionState` extension
  to express its interaction state.
- Motion or animation requires a style state that cannot be expressed
  by `StateStyle` overlays.
- A real consumer needs both global and target-local pointer coordinates
  in one handler. In that case add an explicit API instead of reverting
  widgets to window-relative math.
- Accessibility range or checked state needs richer value types than
  the current `Signal<bool>` and `Signal<f32>` props.

## References

- `crates/sigil/src/interaction.rs` - shared widget interaction state.
- `crates/sigil/src/widgets/` - built-in widget implementations.
- `crates/sigil-events/src/router.rs` - pointer target-local dispatch.
- `examples/vertical-slice/src/main.rs` - all four widgets on one screen.
- `docs/plans/widget-pack-1-checkbox-toggle-slider.md` - implementation plan.
- Commits `4ecb098` (substrate polish + TextStyle relocation) and `3e5af78` (widget pack: checkbox, toggle, slider; `InteractionState::dragging` delta; local-coords router refactor).
