#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use std::cell::Cell;
use std::rc::Rc;

use sigil::Host;
use sigil::app::FrameHooks;
use sigil::events::{
    EventRouter, Key, KeyEvent, KeyEventInit, KeyPhase, LogicalKey, ModifiersState,
};
use sigil::events::{PRIMARY, PhysicalKey, ShellEvent, Shortcut};

use sigil::prelude::*;
use sigil::reactive::Runtime;
use sigil::tasks::Status;
use sigil::widgets::{SortDir, SortKey};

const VIEW: (f32, f32) = (480.0, 320.0);

fn settle(host: &mut Host, router: &mut EventRouter) {
    for _ in 0..3 {
        let _ = host.frame(router, VIEW, 1.0 / 60.0, FrameHooks::default());
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
    Runtime::scope("primitives::switch", || {
        let page = signal(0_u8);
        let mut host = Host::new();
        let mut router = EventRouter::new();
        let view = counted(&mounts, "First page");
        host.mount(
            column().child(
                switch(page)
                    .case(0, view)
                    .case(1, || text("Second page"))
                    .keep_alive(keep_alive),
            ),
        );
        settle(&mut host, &mut router);
        assert!(drawn(&mut host, "First page"));
        page.set(1);
        settle(&mut host, &mut router);
        assert!(drawn(&mut host, "Second page"));
        assert!(!drawn(&mut host, "First page"));
        page.set(0);
        settle(&mut host, &mut router);
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
fn match_status_shows_the_view_of_each_status() {
    Runtime::scope("primitives::match_status", || {
        let step = signal(0_u8);
        let status = move || match step.get() {
            0 => Status::<u32, String>::Pending,
            1 => Status::Ok(3),
            _ => Status::Err(String::from("offline")),
        };
        let mut host = Host::new();
        let mut router = EventRouter::new();
        host.mount(
            column().child(
                match_status(status)
                    .pending(|| text("Loading"))
                    .ok(|value: &u32| text(format!("Loaded {value}")))
                    .error(|error: &String| text(format!("Failed: {error}"))),
            ),
        );
        settle(&mut host, &mut router);
        assert!(drawn(&mut host, "Loading"));

        step.set(1);
        settle(&mut host, &mut router);
        assert!(drawn(&mut host, "Loaded 3"));

        step.set(2);
        settle(&mut host, &mut router);
        assert!(drawn(&mut host, "Failed: offline"));
    });
}

#[test]
fn tabs_follow_their_labels_and_clamp_the_active_tab() {
    Runtime::scope("primitives::tabs", || {
        let labels = signal(vec![String::from("Alpha"), String::from("Beta")]);
        let active = signal(1_usize);
        let mut host = Host::new();
        let mut router = EventRouter::new();
        host.mount(column().child(tabs().active(active).labels(labels)));
        settle(&mut host, &mut router);
        assert!(drawn(&mut host, "Beta"));

        labels.set(vec![
            String::from("Alpha"),
            String::from("Beta"),
            String::from("Gamma"),
        ]);
        settle(&mut host, &mut router);
        assert!(drawn(&mut host, "Gamma"));

        active.set(2);
        labels.set(vec![String::from("Alpha")]);
        settle(&mut host, &mut router);
        assert!(!drawn(&mut host, "Gamma"));
        assert_eq!(active.get(), 0);
    });
}

fn key_down(key: Key) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: LogicalKey::Named(key),
        physical: PhysicalKey::ArrowRight,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

#[test]
fn a_resizable_sidebar_grows_from_its_handle_keys() {
    Runtime::scope("primitives::sidebar", || {
        let mut host = Host::new();
        let mut router = EventRouter::new();
        host.mount(row().child(sidebar(text("Files")).width(200.0).resizable(120.0, 400.0)));
        settle(&mut host, &mut router);
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
            &mut router,
            VIEW,
            1.0 / 60.0,
            FrameHooks {
                input: Some(Box::new(move |target, events| {
                    events.set_focus(target.cx(), Some(handle));
                    target.route(events, key_down(Key::ArrowRight));
                    None
                })),
                ..FrameHooks::default()
            },
        );
        settle(&mut host, &mut router);
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
    Runtime::scope("primitives::table", || {
        let state = procs();
        state.cycle_sort();
        assert_eq!(
            state.sort_key().get(),
            Some(SortKey {
                column: "name",
                dir: SortDir::Ascending
            })
        );
        state.cycle_sort();
        assert_eq!(
            state.sort_key().get(),
            Some(SortKey {
                column: "cpu",
                dir: SortDir::Ascending
            })
        );
        state.reverse_sort();
        assert_eq!(
            state.sort_key().get(),
            Some(SortKey {
                column: "cpu",
                dir: SortDir::Descending
            })
        );
        state.cycle_sort();
        assert_eq!(state.sort_key().get().map(|key| key.column), Some("name"));

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

#[test]
fn primary_shortcuts_use_the_platform_modifier() {
    let save = Shortcut::primary(LogicalKey::character("s"));
    assert_eq!(save.modifiers, PRIMARY);
    if cfg!(target_os = "macos") {
        assert_eq!(PRIMARY, ModifiersState::META);
    } else {
        assert_eq!(PRIMARY, ModifiersState::CONTROL);
    }
}

#[cfg(feature = "menu")]
#[test]
fn one_shortcut_drives_the_native_menu_hint() {
    let save = Shortcut::primary(LogicalKey::character("s"));
    assert!(sigil::app::menu::accelerator(&save).is_some());
    assert!(sigil::app::menu::accelerator(&Shortcut::plain(Key::F13)).is_none());
}

#[test]
fn typing_in_the_command_menu_filters_its_items() {
    Runtime::scope("primitives::palette", || {
        let mut host = Host::new();
        let mut router = EventRouter::new();
        host.mount(
            column().child(
                command_menu()
                    .item("Attack", || {})
                    .item("Defend", || {})
                    .item("Wait", || {}),
            ),
        );
        settle(&mut host, &mut router);
        assert!(drawn(&mut host, "Attack"));

        let _ = host.frame(
            &mut router,
            VIEW,
            1.0 / 60.0,
            FrameHooks {
                input: Some(Box::new(|target, events| {
                    for ch in ["w", "a"] {
                        target.route(
                            events,
                            ShellEvent::Key(KeyEvent::new(KeyEventInit {
                                key: LogicalKey::character(ch),
                                physical: PhysicalKey::KeyA,
                                modifiers: ModifiersState::empty(),
                                repeat: false,
                                phase: KeyPhase::Down,
                                text: Some(ch.into()),
                            })),
                        );
                    }
                    None
                })),
                ..FrameHooks::default()
            },
        );
        settle(&mut host, &mut router);

        assert!(drawn(&mut host, "Wait"));
        assert!(!drawn(&mut host, "Attack"));
        assert!(!drawn(&mut host, "Defend"));
    });
}
