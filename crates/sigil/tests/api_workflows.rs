use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use sigil::Host;
use sigil::app::{FrameHooks, HostMode};
use sigil::layout::TrackSize;
use sigil::prelude::*;
use sigil::reactive::{EventChannel, Owner};
use sigil::style::{ColorScale, ColorValue};
use sigil::widgets::{DiffView, Disclosure, Patch};
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
}

#[test]
fn a_frame_reports_a_signal_write_and_then_goes_idle() {
    Owner::scope("api_workflows::frame", || {
        let label = signal(String::from("first"));
        let mut host = Host::new(HostMode::Pixel);
        host.mount(move || column().child(text(move || label.get())));
        let first = host.frame(VIEW, DT, FrameHooks::default());
        assert!(first.layout().is_ok() && !first.exit());
        let _settle = host.frame(VIEW, DT, FrameHooks::default());
        let idle = host.frame(VIEW, DT, FrameHooks::default());
        assert!(!idle.dirty(), "an idle frame asked for a repaint");

        label.set(String::from("second"));
        let seen = Cell::new(None);
        let hooks = FrameHooks::default().paint(|_, frame| seen.set(Some(frame.dirty())));
        let between = host.frame(VIEW, DT, hooks);
        assert!(
            between.dirty(),
            "a write between frames did not mark the frame dirty"
        );
        assert_eq!(seen.get(), Some(true), "the paint hook saw another frame");
        assert!(host.emit(VIEW).find_text("second").is_some());

        let write = FrameHooks::default().input(|_| {
            label.set(String::from("third"));
            None
        });
        assert!(
            host.frame(VIEW, DT, write).dirty(),
            "a write in the input hook"
        );
        let _again = host.frame(VIEW, DT, FrameHooks::default());
        assert!(
            !host.frame(VIEW, DT, FrameHooks::default()).dirty(),
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

#[test]
fn reactive_scalars_and_builder_setters_update_live() {
    Owner::scope("api_workflows::reactive_setters", || {
        let seg = signal(0_usize);
        let dd = signal(0_usize);
        let sel = signal(false);
        let disc = signal(Disclosure::Collapsed);
        let view = signal(DiffView::Unified);
        let mut ui = Harness::new(move || {
            let patch = signal(Patch::parse("--- a/x\n+++ b/x\n@@ -1 +1 @@\n-old\n+new"));
            column()
                .child(badge(5_usize, Tone::Info))
                .child(segmented(seg, ["A", "B"]))
                .child(dropdown(dd, ["One", "Two"]).width(160.0))
                .child(list_row("svc").selected(sel).disclosure(disc))
                .child(diff(patch).view(view))
        })
        .size(80, 24);
        ui.expect_text("5");
        ui.click("B");
        assert_eq!(seg.peek(), 1);
        sel.set(true);
        disc.set(Disclosure::Expanded);
        view.set(DiffView::Split);
        ui.settle();
        assert_eq!(
            ui.get((Role::GenericContainer, "svc")).is_expanded(),
            Some(true)
        );
    });
}

#[test]
fn password_and_composite_fields_and_interactive_chips_expose_a11y() {
    Owner::scope("api_workflows::field_chip_a11y", || {
        let pw = signal(String::new());
        let md = signal(String::new());
        let hits = signal(0_u32);
        let mut ui = Harness::new(move || {
            column()
                .gap(Space::Sm)
                .child(field("Secret", text_field(pw).password()))
                .child(field("Notes", markdown_edit(md)))
                .child(chip("Degraded", Tone::Warning).activate(move || hits.update(|n| *n += 1)))
        })
        .size(80, 24);
        ui.type_into("Secret", "s3cr3t");
        assert_eq!(pw.peek(), "s3cr3t");
        ui.type_into("Notes", "hello");
        assert!(md.peek().contains("hello"));
        ui.click("Degraded");
        assert_eq!(hits.peek(), 1);
    });
}

fn shell_work_area(keys: Signal<String>) -> NodeBuilder {
    let active = signal(0_usize);
    let body = column()
        .fill()
        .min_height(0.0)
        .child(
            grid([TrackSize::Fr(1.0), TrackSize::Fr(1.0)])
                .fill_width()
                .child(text("L"))
                .child(text("R")),
        )
        .child(divider(Orientation::Horizontal))
        .child(tabs().active(active).tab("Tab1", move || {
            button("Ping").activate(move || keys.set(String::from("clicked")))
        }))
        .child(hint_bar("?", "Ready"));
    splitter(Orientation::Horizontal)
        .pane(text("Nav"))
        .width(120.0)
        .pane(body)
        .grow(1.0)
        .commit()
        .fill()
        .into()
}

#[test]
fn app_shell_splitter_grid_divider_tabs_and_hints_fit_cell_host() {
    Owner::scope("api_workflows::app_shell_fit", || {
        let keys = signal(String::new());
        let mut ui = Harness::new(move || {
            let open = signal(false);
            app_shell(
                menu_bar().menu("File", vec![MenuItem::new("Quit")]),
                shell_work_area(keys),
                status_bar([text("OK").nowrap()]),
                Some(modal(open).title("M")),
            )
            .shortcut(Shortcut::primary('k'), move |_| {
                keys.set(String::from("cmd-k"));
            })
            .key_any(move |e| {
                if *e.key() == '?' {
                    keys.set(String::from("help"));
                }
            })
        })
        .size(120, 36);
        ui.click("Ping");
        assert_eq!(keys.peek(), "clicked");
        ui.press(Shortcut::primary('k'));
        assert_eq!(keys.peek(), "cmd-k");
        ui.press(Shortcut::plain('?'));
        assert_eq!(keys.peek(), "help");
        ui.assert_no_flaws();
    });
}

#[test]
fn theme_transition_animates_cleanly_across_frames() {
    let mut ui = Harness::new(|| {
        column()
            .padding(Space::Md)
            .background(Token::Surface)
            .transition(
                Transition::colors()
                    .duration(Duration::from_millis(120))
                    .ease(Easing::Linear),
            )
            .child(button("Dark").activate(|| Theme::dark().apply()))
    })
    .pixels(320, 120);
    ui.click("Dark");
    ui.advance(Duration::from_millis(60));
    ui.assert_no_flaws();
}

#[test]
fn power_user_theme_and_status_primitives_compose() {
    let scale = ColorScale::from_steps([Color::WHITE; 12]).on_solid(Color::BLACK);
    assert_eq!(scale.steps().len(), 12);
    assert_eq!(scale.solid_ink(), Color::BLACK);
    assert_eq!(
        (Color::WHITE.alpha(128).a, Token::ALL.len()),
        (128, Token::COUNT)
    );
    assert_eq!(
        ColorValue::from("border-strong"),
        ColorValue::Token(Token::BorderStrong)
    );
    assert_eq!(
        ColorValue::from("brand-border"),
        ColorValue::Custom("brand-border")
    );
    assert_eq!(Status::<i32, &str>::Ok(7).map(|n| n + 1).ok(), Some(8));
}

#[test]
fn power_user_host_and_reactive_channels_compose() {
    let scale = ColorScale::from_steps([Color::WHITE; 12]).on_solid(Color::BLACK);
    Owner::scope("api_workflows::power_user", || {
        let sig = signal(42_i32);
        assert!(sig.is_alive() && sig.try_peek() == Some(42));
        let ch = EventChannel::<i32>::new();
        let seen = Rc::new(Cell::new(0_i32));
        let slot = Rc::clone(&seen);
        let _sub = ch.subscribe(move |v| slot.set(v));
        ch.emit(9);
        assert_eq!(seen.get(), 9);

        let mut host = Host::new(HostMode::Pixel);
        host.apply_theme(Theme::dark().tone(Tone::Primary, scale));
        host.mount(|| {
            let band = signal(2.0_f32..=8.0_f32);
            column()
                .child(markdown("# Heading"))
                .child(progress(0.5))
                .child(gauge(0.75).label("Load"))
                .child(vega(r#"{"mark":"bar"}"#))
                .child(scroll(range_slider(band, 0.0..=10.0).label("Band")).height(48.0))
        });
        let ink = Rc::new(Cell::new(Color::WHITE));
        let ink_slot = Rc::clone(&ink);
        assert!(
            host.frame(
                VIEW,
                DT,
                FrameHooks::default().paint(move |_, _| {
                    ink_slot.set(Theme::with(|t| t.color(Token::OnAccent)));
                }),
            )
            .layout()
            .is_ok()
        );
        let root = host.layout(VIEW).expect("layout ok").expect("mounted root");
        assert!(host.node_bounds(root).is_some() && host.scale() > 0.0);
        assert_eq!(ink.get(), Color::BLACK);
    });
}
