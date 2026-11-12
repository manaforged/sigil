use std::cell::Cell;
use std::ops::RangeInclusive;
use std::rc::Rc;

use sigil::events::Key;
use sigil::geometry::Point;
use sigil::kernel::Role;
use sigil::prelude::*;
use sigil::widgets::range_slider;
use sigil_kittest::Harness;

fn open(start: RangeInclusive<f32>) -> (Harness, Signal<RangeInclusive<f32>>) {
    let slot = Rc::new(Cell::new(None::<Signal<RangeInclusive<f32>>>));
    let export = Rc::clone(&slot);
    let mut ui = Harness::new(move || {
        let price = signal(start.clone());
        export.set(Some(price));
        column().child(range_slider(price, 0.0..=100.0).step(10.0).label("Price"))
    })
    .size(40, 4);
    ui.settle();
    let value = slot.get().expect("range signal");
    (ui, value)
}

fn thumbs(ui: &mut Harness) -> (f64, f64) {
    let low = ui.get_by_role(Role::Slider, "Price minimum").numeric();
    let high = ui.get_by_role(Role::Slider, "Price maximum").numeric();
    (low.expect("minimum value"), high.expect("maximum value"))
}

fn focused(ui: &mut Harness) -> String {
    ui.focused()
        .and_then(|node| node.name().map(str::to_owned))
        .unwrap_or_default()
}

#[test]
fn each_thumb_takes_focus_and_keys_without_passing_the_other() {
    let (mut ui, value) = open(20.0..=80.0);
    assert_eq!(focused(&mut ui), "Price minimum");
    ui.press(Key::ArrowRight).settle();
    assert_eq!(value.peek(), 30.0..=80.0);
    ui.press(Key::End).settle();
    assert_eq!(value.peek(), 80.0..=80.0);
    ui.focus_next();
    assert_eq!(focused(&mut ui), "Price maximum");
    ui.press(Key::Home).settle();
    assert_eq!(value.peek(), 80.0..=80.0);
    ui.press(Key::ArrowRight).settle();
    assert_eq!(value.peek(), 80.0..=90.0);
    assert_eq!(thumbs(&mut ui), (80.0, 90.0));
}

#[test]
fn dragging_moves_the_nearest_thumb_and_stops_at_the_other() {
    let (mut ui, value) = open(20.0..=80.0);
    let track = ui
        .get_by_role(Role::Group, "Price")
        .bounds()
        .expect("slider bounds");
    let y = track.origin.y + track.height / 2.0;
    let left = Point::new(track.origin.x, y);
    let right = Point::new(track.origin.x + track.width - 1.0, y);
    let high = ui.get_by_role(Role::Slider, "Price maximum");
    ui.drag(&high, &left).settle();
    assert_eq!(value.peek(), 20.0..=20.0);
    assert_eq!(focused(&mut ui), "Price maximum");
    ui.click(&right).settle();
    assert_eq!(value.peek(), 20.0..=100.0);
    assert_eq!(thumbs(&mut ui), (20.0, 100.0));
}

#[test]
fn overlapped_thumbs_follow_the_drag_direction() {
    let (mut ui, value) = open(100.0..=100.0);
    let track = ui
        .get_by_role(Role::Group, "Price")
        .bounds()
        .expect("slider bounds");
    let left = Point::new(track.origin.x, track.origin.y + track.height / 2.0);
    let high = ui.get_by_role(Role::Slider, "Price maximum");
    ui.drag(&high, &left).settle();
    assert_eq!(value.peek(), 0.0..=100.0);
    assert_eq!(focused(&mut ui), "Price minimum");
}

#[test]
fn change_reports_the_whole_range_for_either_thumb() {
    let seen = Rc::new(std::cell::RefCell::new(Vec::new()));
    let record = Rc::clone(&seen);
    let mut ui = Harness::new(move || {
        let record = Rc::clone(&record);
        column().child(
            range_slider(signal(20.0_f32..=80.0), 0.0..=100.0)
                .step(10.0)
                .label("Price")
                .change(move |range| record.borrow_mut().push(range)),
        )
    })
    .size(40, 4);
    ui.settle();
    ui.press(Key::ArrowRight).settle();
    ui.focus_next();
    ui.press(Key::ArrowRight).settle();
    assert_eq!(*seen.borrow(), vec![30.0..=80.0, 30.0..=90.0]);
}
