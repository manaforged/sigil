use std::cell::Cell;
use std::rc::Rc;

use sigil::Host;
use sigil::app::FrameHooks;
use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil::widgets::close_popup;
use sigil_kittest::Harness;

const VIEW: (f32, f32) = (400.0, 200.0);
const DT: f32 = 1.0 / 60.0;

#[test]
fn a_text_field_reports_each_loss_of_focus_once() {
    let lost = Rc::new(Cell::new(0_u32));
    let count = Rc::clone(&lost);
    let mut ui = Harness::new(move || {
        let count = Rc::clone(&count);
        column()
            .child(field(
                "Name",
                text_field(signal(String::new())).focus_out(move || count.set(count.get() + 1)),
            ))
            .child(field("Email", text_field(signal(String::new()))))
    });
    ui.click("Name");
    assert_eq!(lost.get(), 0, "focus has not left the field yet");
    ui.click("Email");
    assert_eq!(lost.get(), 1);
    ui.click("Name").click("Email");
    assert_eq!(lost.get(), 2);
}

#[test]
fn escape_closes_the_open_dialog_and_nothing_is_left_to_close() {
    let show = Rc::new(Cell::new(None));
    let slot = Rc::clone(&show);
    let mut ui = Harness::new(move || {
        let open = signal(true);
        slot.set(Some(open));
        column()
            .child(text("Page"))
            .child(modal(open).title("Rename").child(button("Apply")))
    })
    .size(60, 16);
    ui.expect_text("Rename");
    let open = show.get().expect("the app ran");
    ui.press(NamedKey::Escape);
    assert!(!open.peek(), "Escape left the dialog open");
    assert!(!ui.text().contains("Rename"), "{}", ui.text());
    assert!(!close_popup(), "no popup is open after Escape");
}

#[test]
fn a_frame_reports_a_signal_write_and_then_goes_idle() {
    Owner::scope("api_workflows::frame", || {
        let label = signal(String::from("first"));
        let mut host = Host::new(HostMode::Pixel);
        host.mount(move || column().child(text(move || label.get())));
        let first = host.frame(VIEW, DT, FrameHooks::default());
        assert!(first.layout.is_ok() && !first.exit);
        let _settle = host.frame(VIEW, DT, FrameHooks::default());
        let idle = host.frame(VIEW, DT, FrameHooks::default());
        assert!(!idle.dirty, "an idle frame asked for a repaint");

        label.set(String::from("second"));
        let seen = Cell::new(None);
        let hooks = FrameHooks::default().paint(|_, frame| seen.set(Some(frame.dirty)));
        let between = host.frame(VIEW, DT, hooks);
        assert!(
            between.dirty,
            "a write between frames did not mark the frame dirty"
        );
        assert_eq!(seen.get(), Some(true), "the paint hook saw another frame");
        assert!(host.emit(VIEW).find_text("second").is_some());

        let write = FrameHooks::default().input(|_| {
            label.set(String::from("third"));
            None
        });
        assert!(
            host.frame(VIEW, DT, write).dirty,
            "a write in the input hook"
        );
        let _again = host.frame(VIEW, DT, FrameHooks::default());
        assert!(
            !host.frame(VIEW, DT, FrameHooks::default()).dirty,
            "idle again"
        );
    });
}

#[test]
fn a_list_row_shows_each_part_that_is_set() {
    let mut ui = Harness::new(|| {
        column()
            .child(
                list_row("deploy.yml")
                    .marker("M")
                    .detail("workflows")
                    .status("failed")
                    .selected(true),
            )
            .child(list_row("README.md"))
    })
    .size(60, 6);
    let screen = ui.text();
    let row = screen
        .lines()
        .find(|line| line.contains("deploy.yml"))
        .unwrap_or_default();
    let at = ["M", "deploy.yml", "workflows", "failed"].map(|part| row.find(part));
    assert!(at.is_sorted() && at.iter().all(Option::is_some), "{screen}");
    let plain = screen
        .lines()
        .find(|line| line.contains("README.md"))
        .unwrap_or_default();
    assert!(plain.contains("README.md"), "{screen}");
    assert!(
        !plain.contains("failed") && !plain.contains("workflows"),
        "{plain}"
    );
}
