use sigil::style::{StateSlot, StateStyle, StyleAtom, StyleState};

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
    let style = StateStyle::EMPTY
        .slot(StateSlot::Hovered, HOVER)
        .slot(StateSlot::Pressed, PRESS);

    assert_eq!(style.get(StateSlot::Rest), &[] as &[StyleAtom]);
    assert_eq!(style.get(StateSlot::Focused), &[] as &[StyleAtom]);
    assert_eq!(style.get(StateSlot::Disabled), &[] as &[StyleAtom]);
    assert_eq!(style.get(StateSlot::Hovered), HOVER);
    assert_eq!(style.get(StateSlot::Pressed), PRESS);

    let refined = style.slot(StateSlot::Hovered, FOCUS);
    assert_eq!(refined.get(StateSlot::Hovered), FOCUS);
    assert_eq!(refined.get(StateSlot::Pressed), PRESS);
}

#[test]
fn state_style_default_is_empty() {
    let style: StateStyle = StateStyle::default();
    assert_eq!(style, StateStyle::EMPTY);
    assert_eq!(style.resolve(StyleState::default()), &[] as &[StyleAtom]);
}
