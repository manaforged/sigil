use std::cell::Cell;
use std::rc::Rc;

use accesskit::{Invalid, Role as AkRole};
use sigil::events::Key;
use sigil::kernel::Role;
use sigil::prelude::*;
use sigil::style::{Locale, StringKey, Strings, Theme};
use sigil::widgets::{Date, DateTime, Time, TimeValue, time_field};
use sigil_kittest::{Harness, Query};

const fn time(hour: u8, minute: u8, second: u8) -> Time {
    Time::new(hour, minute, second).expect("valid time")
}

fn open<T: TimeValue>(locale: Locale, start: T, seconds: bool) -> (Harness, Signal<T>) {
    open_in(Theme::light().with_locale(locale), start, seconds)
}

fn open_in<T: TimeValue>(theme: Theme, start: T, seconds: bool) -> (Harness, Signal<T>) {
    let slot = Rc::new(Cell::new(None::<Signal<T>>));
    let export = Rc::clone(&slot);
    let mut ui = Harness::new(move || {
        theme.clone().apply();
        let value = signal(start.clone());
        export.set(Some(value));
        field("Alarm", time_field(value).seconds(seconds))
    })
    .size(48, 6);
    ui.settle();
    let value = slot.get().expect("value signal");
    (ui, value)
}

fn shown(ui: &mut Harness) -> String {
    ui.get(&Query::role(Role::TextInput))
        .value()
        .unwrap_or_default()
        .to_owned()
}

fn retype(ui: &mut Harness, text: &str) {
    let input = ui.get(&Query::role(Role::TextInput));
    ui.click(&input).press(Key::End);
    for _ in 0..24 {
        ui.press(Key::Backspace);
    }
    ui.type_text(text).press(Key::Enter).settle();
}

#[test]
fn a_time_field_edits_a_24_hour_time() {
    let (mut ui, value) = open(Locale::de(), time(7, 30, 0), false);
    assert_eq!(shown(&mut ui), "07:30");
    retype(&mut ui, "18:05");
    assert_eq!(value.peek(), time(18, 5, 0));
    assert_eq!(shown(&mut ui), "18:05");
}

#[test]
fn a_time_field_uses_the_12_hour_clock_of_the_locale() {
    let (mut ui, value) = open(Locale::en(), time(7, 30, 0), true);
    assert_eq!(shown(&mut ui), "7:30:00 AM");
    retype(&mut ui, "6:15 pm");
    assert_eq!(value.peek(), time(18, 15, 0));
}

#[test]
fn a_12_hour_clock_shows_and_reads_the_day_periods_of_the_theme_strings() {
    let arabic = Strings::english()
        .with(StringKey::Am, "\u{635}")
        .with(StringKey::Pm, "\u{645}");
    let theme = Theme::light()
        .with_locale(Locale::ar())
        .with_strings(arabic);
    let (mut ui, value) = open_in(theme, time(18, 5, 0), false);
    assert_eq!(shown(&mut ui), "\u{666}:\u{660}\u{665} \u{645}");
    retype(&mut ui, "\u{667}:\u{663}\u{660} \u{635}");
    assert_eq!(value.peek(), time(7, 30, 0));
}

#[test]
fn a_time_set_by_the_app_rewrites_the_text() {
    let (mut ui, value) = open(Locale::de(), time(7, 30, 0), false);
    retype(&mut ui, "18:05");
    value.set(time(9, 0, 0));
    ui.settle();
    assert_eq!(shown(&mut ui), "09:00");
}

#[test]
fn a_time_field_keeps_the_value_and_the_text_of_an_impossible_time() {
    let (mut ui, value) = open(Locale::de(), time(7, 30, 0), false);
    retype(&mut ui, "25:00");
    assert_eq!(value.peek(), time(7, 30, 0));
    assert_eq!(shown(&mut ui), "25:00");
}

fn input_invalid(ui: &mut Harness) -> Option<Invalid> {
    ui.accessibility_tree()
        .nodes
        .into_iter()
        .find(|(_, node)| node.role() == AkRole::TextInput)
        .and_then(|(_, node)| node.invalid())
}

#[test]
fn a_screen_reader_hears_that_an_impossible_time_is_invalid() {
    let (mut ui, _) = open(Locale::de(), time(7, 30, 0), false);
    assert_eq!(input_invalid(&mut ui), None);
    retype(&mut ui, "25:00");
    assert_eq!(input_invalid(&mut ui), Some(Invalid::True));
    retype(&mut ui, "18:05");
    assert_eq!(input_invalid(&mut ui), None);
}

#[test]
fn a_date_and_time_field_edits_both_parts() {
    let start = DateTime::new(Date::new(2026, 11, 11).expect("valid date"), time(9, 0, 0));
    let (mut ui, value) = open(Locale::default(), start, false);
    assert_eq!(shown(&mut ui), "2026-11-11 09:00");
    retype(&mut ui, "2026-12-24 20:30");
    assert_eq!(
        value.peek(),
        DateTime::new(
            Date::new(2026, 12, 24).expect("valid date"),
            time(20, 30, 0)
        )
    );
}

#[test]
fn a_twelve_hour_clock_formats_any_hour_without_panicking() {
    let (locale, strings) = (Locale::en(), Strings::english());
    assert_eq!(
        locale.format_time(250, 0, None, &strings),
        locale.format_time(10, 0, None, &strings)
    );
}
