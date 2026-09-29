use super::{CompositionInput, KeyRoute, composition_events, key_route};
use crate::events::ImeEvent;

#[test]
fn a_finished_composition_clears_the_preedit_then_commits() {
    assert_eq!(
        composition_events(CompositionInput::End("日本")),
        vec![
            ImeEvent::Preedit("".into(), None),
            ImeEvent::Commit("日本".into())
        ]
    );
}

#[test]
fn composing_keys_stay_with_the_browser() {
    assert_eq!(key_route("a", 229, false), KeyRoute::Browser);
    assert_eq!(key_route("Process", 0, false), KeyRoute::Browser);
    assert_eq!(key_route("a", 65, true), KeyRoute::Browser);
    assert_eq!(key_route("a", 65, false), KeyRoute::Host);
}

#[test]
fn a_full_japanese_composition_maps_to_the_desktop_event_sequence() {
    let events: Vec<ImeEvent> = [
        CompositionInput::Start,
        CompositionInput::Update("にほん"),
        CompositionInput::Update("日本"),
        CompositionInput::End("日本"),
    ]
    .into_iter()
    .flat_map(composition_events)
    .collect();
    assert_eq!(
        events,
        vec![
            ImeEvent::Enabled,
            ImeEvent::Preedit("にほん".into(), Some((9, 9))),
            ImeEvent::Preedit("日本".into(), Some((6, 6))),
            ImeEvent::Preedit("".into(), None),
            ImeEvent::Commit("日本".into()),
        ]
    );
    assert_eq!(
        composition_events(CompositionInput::Text("한")),
        vec![ImeEvent::Enabled, ImeEvent::Commit("한".into())]
    );
    assert!(composition_events(CompositionInput::Text("")).is_empty());
}
