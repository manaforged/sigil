use super::*;
use sigil::events::EventRouter;
use sigil::kernel::Cx;

#[test]
fn click_focuses_surface_then_keys_reach_on_key() {
    sigil::reactive::Runtime::scope("code-edit-focus", || {
        let lines = signal(vec!["abc".to_string()]);
        let caret = signal((0_usize, 0_usize));
        let keys_seen = Rc::new(Cell::new(0_u32));
        let recorder = Rc::clone(&keys_seen);
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(580.0, 400.0))
                .child(code_edit(lines, caret).key(move |_event| {
                    recorder.set(recorder.get() + 1);
                })),
        );
        let layout = layout_in(&mut cx, root, 580.0, 400.0);
        let editor = first_text_input(&cx, root);
        let mut router = EventRouter::new();

        router.route(&mut cx, &layout, char_key("a"));
        assert_eq!(
            keys_seen.get(),
            0,
            "unfocused surface must not receive keys"
        );

        router.route(&mut cx, &layout, pointer_down(10.0, 10.0));
        assert_eq!(
            router.focus(),
            Some(editor),
            "clicking the code surface must focus it",
        );

        router.route(&mut cx, &layout, char_key("x"));
        assert_eq!(
            keys_seen.get(),
            1,
            "a focused code surface must forward keys to on_key",
        );
    });
}

#[test]
fn caret_renders_even_when_caret_line_is_past_the_end() {
    sigil::reactive::Runtime::scope("code-edit-caret-clamp", || {
        let lines = signal(vec!["abc".to_string()]);
        let caret = signal((9_usize, 1_usize));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(580.0, 400.0))
                .child(code_edit(lines, caret)),
        );
        let layout = layout_in(&mut cx, root, 580.0, 400.0);
        assert!(
            caret_bar_present(&cx, &layout, root),
            "an out-of-range caret line must clamp to a real row so the caret still draws",
        );
    });
}

#[test]
fn trapped_tab_reaches_on_key_and_does_not_move_focus() {
    sigil::reactive::Runtime::scope("code-edit-tab-trap", || {
        let lines = signal(vec!["abc".to_string()]);
        let caret = signal((0_usize, 0_usize));
        let keys_seen = Rc::new(Cell::new(0_u32));
        let recorder = Rc::clone(&keys_seen);
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(580.0, 400.0))
                .child(code_edit(lines, caret).key(move |_event| {
                    recorder.set(recorder.get() + 1);
                }))
                .child(
                    container()
                        .role(Role::Button)
                        .focusable(true)
                        .layout(LayoutStyle::block().fixed_size(100.0, 20.0)),
                ),
        );
        let layout = layout_in(&mut cx, root, 580.0, 400.0);
        let editor = first_text_input(&cx, root);
        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(editor));

        router.route(
            &mut cx,
            &layout,
            named_key(Key::Tab, ModifiersState::empty()),
        );

        assert_eq!(
            keys_seen.get(),
            1,
            "a trap-tab surface must receive the raw Tab (to indent), not lose it to focus",
        );
        assert_eq!(
            router.focus(),
            Some(editor),
            "plain Tab on a trap-tab surface must not move focus away",
        );
    });
}

#[test]
fn shift_tab_still_escapes_a_trap_tab_surface() {
    sigil::reactive::Runtime::scope("code-edit-tab-escape", || {
        let lines = signal(vec!["abc".to_string()]);
        let caret = signal((0_usize, 0_usize));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(580.0, 400.0))
                .child(code_edit(lines, caret).key(move |_event| {}))
                .child(
                    container()
                        .role(Role::Button)
                        .focusable(true)
                        .layout(LayoutStyle::block().fixed_size(100.0, 20.0)),
                ),
        );
        let layout = layout_in(&mut cx, root, 580.0, 400.0);
        let editor = first_text_input(&cx, root);
        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(editor));

        router.route(&mut cx, &layout, named_key(Key::Tab, ModifiersState::SHIFT));

        assert!(
            router.focus().is_some() && router.focus() != Some(editor),
            "Shift+Tab must move focus off a trap-tab surface (no keyboard trap)",
        );
    });
}

#[test]
fn read_only_surface_does_not_trap_tab() {
    sigil::reactive::Runtime::scope("code-edit-readonly-tab", || {
        let lines = signal(vec!["abc".to_string()]);
        let caret = signal((0_usize, 0_usize));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(580.0, 400.0))
                .child(code_edit(lines, caret))
                .child(
                    container()
                        .role(Role::Button)
                        .focusable(true)
                        .layout(LayoutStyle::block().fixed_size(100.0, 20.0)),
                ),
        );
        let layout = layout_in(&mut cx, root, 580.0, 400.0);
        let editor = first_text_input(&cx, root);
        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(editor));

        router.route(
            &mut cx,
            &layout,
            named_key(Key::Tab, ModifiersState::empty()),
        );

        assert!(
            router.focus().is_some() && router.focus() != Some(editor),
            "a read-only code surface must let Tab traverse focus, not trap it",
        );
    });
}
