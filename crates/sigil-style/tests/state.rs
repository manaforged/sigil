#![allow(missing_docs)]

use allocation_counter as _;
use criterion as _;
use phf as _;
use sigil_core as _;
use sigil_hal as _;
use sigil_reactive as _;
use sigil_style_macros as _;
use smallvec as _;
use strsim as _;

use sigil_style::{StateSlot, StateStyle, StyleAtom, StyleState};

static REST: &[StyleAtom] = &[StyleAtom::Opacity(10_000)];
static FOCUS: &[StyleAtom] = &[StyleAtom::Opacity(9_500)];
static HOVER: &[StyleAtom] = &[StyleAtom::Opacity(9_000)];
static PRESS: &[StyleAtom] = &[StyleAtom::Opacity(8_000)];
static DISABLED: &[StyleAtom] = &[StyleAtom::Opacity(5_000)];

#[test]
fn state_slot_priority_is_deterministic() {
    assert_eq!(
        StyleState {
            disabled: true,
            hovered: true,
            pressed: true,
            focused: true,
        }
        .slot(),
        StateSlot::Disabled
    );
    assert_eq!(
        StyleState {
            disabled: false,
            hovered: true,
            pressed: true,
            focused: true,
        }
        .slot(),
        StateSlot::Pressed
    );
    assert_eq!(
        StyleState {
            disabled: false,
            hovered: true,
            pressed: false,
            focused: true,
        }
        .slot(),
        StateSlot::Hovered
    );
}

#[test]
fn state_style_resolves_overlay_slice() {
    let style = StateStyle::new(REST, FOCUS, HOVER, PRESS, DISABLED);

    assert_eq!(style.resolve(StyleState::default()), REST);
    assert_eq!(
        style.resolve(StyleState {
            disabled: false,
            hovered: false,
            pressed: false,
            focused: true,
        }),
        FOCUS
    );
}

#[test]
fn state_style_builders_mutate_only_named_slot() {
    // Incremental construction: EMPTY + with_* must touch only the slot
    // named, leaving everything else empty. Mirrors how downstream widgets
    // want to extend a state style without re-declaring every slot.
    let style = StateStyle::EMPTY.with_hovered(HOVER).with_pressed(PRESS);

    assert_eq!(style.rest(), &[] as &[StyleAtom]);
    assert_eq!(style.focused(), &[] as &[StyleAtom]);
    assert_eq!(style.disabled(), &[] as &[StyleAtom]);
    assert_eq!(style.hovered(), HOVER);
    assert_eq!(style.pressed(), PRESS);

    // Replacing a slot is write-last-wins, not append.
    let refined = style.with_hovered(FOCUS);
    assert_eq!(refined.hovered(), FOCUS);
    assert_eq!(refined.pressed(), PRESS);
}

#[test]
fn state_style_default_is_empty() {
    let style: StateStyle = StateStyle::default();
    assert_eq!(style, StateStyle::EMPTY);
    assert_eq!(style.resolve(StyleState::default()), &[] as &[StyleAtom]);
}
