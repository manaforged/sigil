use super::*;
use crate::{ColorAtom, Rgba8};

#[test]
fn new_overlay_is_empty() {
    let overlay = StyleOverlay::new();
    assert!(overlay.is_empty());
    assert!(overlay.atoms_for(StateSlot::Hovered).is_empty());
    assert!(overlay.atoms_for(StateSlot::Rest).is_empty());
}

#[test]
fn atoms_for_returns_matching_slot() {
    let red = Rgba8 {
        r: 255,
        g: 0,
        b: 0,
        a: 255,
    };
    let mut hover = SmallVec::new();
    hover.push(StyleAtom::BackgroundColor(ColorAtom::Rgba8(red)));
    let overlay = StyleOverlay::from_slots(
        SmallVec::new(),
        hover.clone(),
        SmallVec::new(),
        SmallVec::new(),
        SmallVec::new(),
    );
    assert!(!overlay.is_empty());
    assert_eq!(overlay.atoms_for(StateSlot::Hovered), hover.as_slice());
    assert!(overlay.atoms_for(StateSlot::Pressed).is_empty());
    assert!(overlay.atoms_for(StateSlot::Rest).is_empty());
}
