#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use std::cell::Cell;
use std::rc::Rc;

use sigil::Host;
use sigil::app::FrameHooks;
use sigil::events::{Event, PhysicalKey, Shortcut};
use sigil::events::{Key, KeyEvent, KeyPhase, Modifiers, NamedKey};

use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil::widgets::{SortDirection, SortKey};

const VIEW: (f32, f32) = (480.0, 320.0);

fn settle(host: &mut Host) {
    for _ in 0..3 {
        let _ = host.frame(VIEW, 1.0 / 60.0, FrameHooks::default());
    }
}

fn drawn(host: &mut Host, label: &str) -> bool {
    host.emit(VIEW).find_text(label).is_some()
}

fn counted(count: &Rc<Cell<u32>>, label: &'static str) -> impl Fn() -> NodeBuilder + 'static {
    let count = Rc::clone(count);
    move || {
        count.set(count.get() + 1);
        container().child(text(label))
    }
}

fn mounts_of_first_page(keep_alive: bool) -> u32 {
    let mounts = Rc::new(Cell::new(0));
    Owner::scope("primitives::switch", || {
        let page = signal(0_u8);
        let mut host = Host::new(HostMode::Pixel);
        let view = counted(&mounts, "First page");
        host.mount(
            column().child(
                switch(page)
                    .case(0, view)
                    .case(1, || text("Second page"))
                    .keep_alive(keep_alive),
            ),
        );
        settle(&mut host);
        assert!(drawn(&mut host, "First page"));
        page.set(1);
        settle(&mut host);
        assert!(drawn(&mut host, "Second page"));
        assert!(!drawn(&mut host, "First page"));
        page.set(0);
        settle(&mut host);
        assert!(drawn(&mut host, "First page"));
    });
    mounts.get()
}

#[test]
fn switch_keeps_a_page_mounted_only_with_keep_alive() {
    assert_eq!(mounts_of_first_page(true), 1);
    assert_eq!(mounts_of_first_page(false), 2);
}

#[test]
fn tabs_follow_their_labels_and_clamp_the_active_tab() {
    Owner::scope("primitives::tabs", || {
        let labels = signal(vec![String::from("Alpha"), String::from("Beta")]);
        let active = signal(1_usize);
        let mut host = Host::new(HostMode::Pixel);
        host.mount(column().child(tabs().active(active).labels(labels)));
        settle(&mut host);
        assert!(drawn(&mut host, "Beta"));

        labels.set(vec![
            String::from("Alpha"),
            String::from("Beta"),
            String::from("Gamma"),
        ]);
        settle(&mut host);
        assert!(drawn(&mut host, "Gamma"));

        active.set(2);
        labels.set(vec![String::from("Alpha")]);
        settle(&mut host);
        assert!(!drawn(&mut host, "Gamma"));
        assert_eq!(active.get(), 0);
    });
}

fn key_down(key: NamedKey) -> Event {
    Event::key(Key::Named(key), KeyPhase::Down).physical(PhysicalKey::ArrowRight)
}

#[test]
fn a_resizable_sidebar_grows_from_its_handle_keys() {
    Owner::scope("primitives::sidebar", || {
        let mut host = Host::new(HostMode::Pixel);
        host.mount(row().child(sidebar(text("Files")).width(200.0).resizable(120.0, 400.0)));
        settle(&mut host);
        let handle = {
            let root = host.root().expect("mounted");
            let cx = host.cx();
            let mut found = None;
            cx.visit_preorder(root, |id, node| {
                let name = match &node.a11y.name {
                    sigil::kernel::AccessibleName::Static(name) => name.clone(),
                    sigil::kernel::AccessibleName::Derived(name) => name.peek(),
                    sigil::kernel::AccessibleName::None => String::new(),
                };
                if name == "Resize sidebar" {
                    found = Some(id);
                }
            });
            found.expect("resize handle")
        };
        let before = host
            .cx()
            .get(handle)
            .and_then(|node| node.a11y.value_now.map(|value| value.get()))
            .expect("handle value");

        let _ = host.frame(
            VIEW,
            1.0 / 60.0,
            FrameHooks::default().input(move |target| {
                let (cx, _, router) = target.parts();
                router.set_focus(cx, Some(handle));
                target.route(key_down(NamedKey::ArrowRight));
                None
            }),
        );
        settle(&mut host);
        let after = host
            .cx()
            .get(handle)
            .and_then(|node| node.a11y.value_now.map(|value| value.get()))
            .expect("handle value");

        assert_eq!(after, before + 16.0);
    });
}

#[derive(Clone)]
struct Proc {
    name: &'static str,
    cpu: u32,
}

fn procs() -> TableState<Proc> {
    TableState::new(
        vec![
            Proc { name: "b", cpu: 2 },
            Proc { name: "a", cpu: 3 },
            Proc { name: "c", cpu: 1 },
        ],
        vec![
            TableColumn::new("name", "Name").sortable(|x: &Proc, y: &Proc| x.name.cmp(y.name)),
            TableColumn::new("pid", "Pid"),
            TableColumn::new("cpu", "CPU").sortable(|x: &Proc, y: &Proc| x.cpu.cmp(&y.cpu)),
        ],
    )
}

#[test]
fn table_sort_cycles_reverses_and_moves_the_selection() {
    Owner::scope("primitives::table", || {
        let state = procs();
        state.next_sort();
        assert_eq!(
            state.sort_key().get(),
            Some(SortKey::new("name", SortDirection::Ascending))
        );
        state.next_sort();
        assert_eq!(
            state.sort_key().get(),
            Some(SortKey::new("cpu", SortDirection::Ascending))
        );
        state.reverse_sort();
        assert_eq!(
            state.sort_key().get(),
            Some(SortKey::new("cpu", SortDirection::Descending))
        );
        state.next_sort();
        assert_eq!(state.sort_key().get().map(|key| key.column()), Some("name"));

        state.select_by(1);
        let first = state.selected_index();
        state.select_by(1);
        let second = state.selected_index();
        state.select_by(10);
        let last = state.selected_index();

        let order = state.visible().get();
        assert_eq!(first, order.first().copied());
        assert_eq!(second, order.get(1).copied());
        assert_eq!(last, order.last().copied());
    });
}

fn key_with(key: &str, modifiers: Modifiers) -> KeyEvent {
    match Event::press(Key::character(key), modifiers) {
        [Event::Key(event), _] => event,
        _ => unreachable!("press starts with a key event"),
    }
}

#[test]
fn shift_letter_shortcuts_match_the_shifted_character() {
    let shortcut: Shortcut = "shift+p".parse().expect("shortcut");
    let shifted = key_with("P", Modifiers::SHIFT);
    assert!(shortcut.matches(&shifted));
    assert!(!shortcut.matches(&key_with("P", Modifiers::empty())));
    assert!(!Shortcut::plain(Key::character("p")).matches(&shifted));
    assert!(!Shortcut::plain(Key::character("p")).matches(&key_with("P", Modifiers::empty())));
}

#[test]
fn primary_shortcuts_use_the_platform_modifier() {
    let save = Shortcut::primary(Key::character("s"));
    if cfg!(target_os = "macos") {
        assert_eq!(save.modifiers(), Modifiers::META);
    } else {
        assert_eq!(save.modifiers(), Modifiers::CTRL);
    }
}

#[test]
fn typing_in_the_command_menu_filters_its_items() {
    Owner::scope("primitives::palette", || {
        let mut host = Host::new(HostMode::Pixel);
        host.mount(
            column().child(
                command_menu()
                    .item(MenuItem::new("Attack").activate(|| {}))
                    .item(MenuItem::new("Defend").activate(|| {}))
                    .item(MenuItem::new("Wait").activate(|| {})),
            ),
        );
        settle(&mut host);
        assert!(drawn(&mut host, "Attack"));

        let _ = host.frame(
            VIEW,
            1.0 / 60.0,
            FrameHooks::default().input(|target| {
                for ch in ["w", "a"] {
                    target.route(
                        Event::key(Key::character(ch), KeyPhase::Down)
                            .physical(PhysicalKey::KeyA)
                            .text(Some(ch.into())),
                    );
                }
                None
            }),
        );
        settle(&mut host);

        assert!(drawn(&mut host, "Wait"));
        assert!(!drawn(&mut host, "Attack"));
        assert!(!drawn(&mut host, "Defend"));
    });
}
