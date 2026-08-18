//! DR-057 D2 `table(state)` view — input→pixel and windowing contract.
//!
//! Covers the header sort cycle (pointer + reorder), selection and
//! hover painting, the 100k-row windowed reactive path (sort without
//! rebuild), the LIVE keyboard row count under quick-filter, wheel
//! scrolling, and the grid a11y channels on root / rows / headers.

#![allow(missing_docs)]
#![allow(unused_crate_dependencies)]
#![allow(clippy::unwrap_used, clippy::too_many_lines)]
// Probe closures and re-laid-out snapshots reuse names (`cx`, `_layout`,
// `selected`) across a test body by design; the shadowing is intentional.
#![allow(clippy::shadow_unrelated)]

use arboard as _;
use criterion as _;
use image as _;
use pollster as _;
use sigil_actions as _;
use sigil_animation as _;
use sigil_animation_runtime as _;
use sigil_app as _;
use sigil_bidi as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_profile as _;
use sigil_reactive as _;
use sigil_style as _;
use sigil_tasks as _;
use wgpu as _;

use sigil::prelude::*;
use sigil::widgets::{SortDir, SortKey, TableState};
use sigil_hal::{KeyEventInit, PhysicalKey, Point, PointerEventInit, ScrollEventInit};
use sigil_renderer::{SceneCommand, emit_scene};

const ROW_H: f32 = 20.0;
const HEADER_H: f32 = 28.0;
const VIEW_H: f32 = 200.0;
const WIDTH: f32 = 200.0;

#[derive(Clone)]
struct Item {
    name: String,
    count: i64,
}

fn item(name: &str, count: i64) -> Item {
    Item {
        name: name.to_owned(),
        count,
    }
}

fn demo_rows() -> Vec<Item> {
    vec![
        item("gamma", 30),
        item("alpha", 10),
        item("delta", 40),
        item("beta", 20),
    ]
}

fn demo_state() -> TableState<Item> {
    TableState::new(
        demo_rows(),
        vec![
            TableColumn::new("name", "Name")
                .sort_by(|row: &Item| row.name.clone())
                .filterable(|row: &Item, query: &str| row.name.contains(query))
                .width(ColWidth::Fixed(100.0))
                .cell(|row: &Item| container().child(text(row.name.clone()))),
            TableColumn::new("count", "Count")
                .sort_by(|row: &Item| row.count)
                .width(ColWidth::Fixed(100.0))
                .cell(|row: &Item| container().child(text(row.count.to_string()))),
        ],
    )
}

struct StubMeasure;
impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::new(40.0, 16.0)
    }
}

fn compute(cx: &mut Cx, root: NodeId) -> LayoutEngine {
    let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
    layout
        .compute(cx, root, Size::fixed(640.0, 480.0))
        .expect("layout");
    layout
}

fn rect_colors(cx: &Cx, root: NodeId) -> Vec<sigil_style::Rgba8> {
    emit_scene(cx, root)
        .commands
        .iter()
        .filter_map(|c| match c {
            SceneCommand::Rect(r) => Some(r.color),
            _ => None,
        })
        .collect()
}

fn glyph_texts(cx: &Cx, root: NodeId) -> Vec<String> {
    emit_scene(cx, root)
        .commands
        .iter()
        .filter_map(|c| match c {
            SceneCommand::GlyphRun(d) => Some(d.run.text.clone()),
            _ => None,
        })
        .collect()
}

const fn pointer(phase: PointerPhase, x: f32, y: f32, modifiers: ModifiersState) -> ShellEvent {
    let (button, buttons) = match phase {
        PointerPhase::Down => (Some(MouseButton::Left), MouseButtons::LEFT),
        _ => (None, MouseButtons::empty()),
    };
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button,
        buttons,
        modifiers,
        phase,
    }))
}

fn click(router: &mut EventRouter, cx: &mut Cx, layout: &LayoutEngine, x: f32, y: f32) {
    router.route(
        cx,
        layout,
        pointer(PointerPhase::Down, x, y, ModifiersState::empty()),
    );
    router.route(
        cx,
        layout,
        pointer(PointerPhase::Up, x, y, ModifiersState::empty()),
    );
}

fn key_down(key: Key, physical: PhysicalKey) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: key.into(),
        physical,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

fn visible_names(state: &TableState<Item>) -> Vec<String> {
    let rows = state.rows().get();
    state
        .visible()
        .get()
        .iter()
        .filter_map(|&index| rows.get(index as usize))
        .map(|row| row.name.clone())
        .collect()
}

#[test]
fn header_click_cycles_sort_and_reorders_rows() {
    sigil::reactive::Runtime::scope("table-view-sort-cycle", || {
        let state = demo_state();
        let mut cx = Cx::new_root();
        let root = cx.add(
            table(state.clone())
                .row_height(ROW_H)
                .header_height(HEADER_H)
                .viewport_width(WIDTH)
                .viewport_height(VIEW_H),
        );
        let layout = compute(&mut cx, root);
        let mut router = EventRouter::new();

        // Header cell "name": x in [0, 100), y in [0, 28).
        click(&mut router, &mut cx, &layout, 50.0, 14.0);
        assert_eq!(
            state.sort_key().get(),
            Some(SortKey {
                column: "name",
                dir: SortDir::Ascending,
            }),
            "first click sorts ascending",
        );
        assert_eq!(visible_names(&state), ["alpha", "beta", "delta", "gamma"]);

        click(&mut router, &mut cx, &layout, 50.0, 14.0);
        assert_eq!(
            state.sort_key().get(),
            Some(SortKey {
                column: "name",
                dir: SortDir::Descending,
            })
        );
        assert_eq!(visible_names(&state), ["gamma", "delta", "beta", "alpha"]);

        click(&mut router, &mut cx, &layout, 50.0, 14.0);
        assert_eq!(state.sort_key().get(), None, "third click clears the sort");
        assert_eq!(visible_names(&state), ["gamma", "alpha", "delta", "beta"]);

        // The second column ("count", x in [100, 200)) sorts too.
        click(&mut router, &mut cx, &layout, 150.0, 14.0);
        assert_eq!(
            state.sort_key().get(),
            Some(SortKey {
                column: "count",
                dir: SortDir::Ascending,
            })
        );
    });
}

#[test]
fn row_click_drives_selection_with_modifiers() {
    sigil::reactive::Runtime::scope("table-view-selection", || {
        let state = demo_state();
        let mut cx = Cx::new_root();
        let root = cx.add(
            table(state.clone())
                .row_height(ROW_H)
                .header_height(HEADER_H)
                .viewport_width(WIDTH)
                .viewport_height(VIEW_H),
        );
        let layout = compute(&mut cx, root);
        let mut router = EventRouter::new();

        // Body row N sits at y = HEADER_H + N * ROW_H .. +ROW_H.
        let row_y = |n: usize| {
            #[allow(clippy::cast_precision_loss)]
            let n = n as f32;
            (n + 0.5).mul_add(ROW_H, HEADER_H)
        };
        router.route(
            &mut cx,
            &layout,
            pointer(PointerPhase::Down, 50.0, row_y(1), ModifiersState::empty()),
        );
        assert!(state.selection().get().contains(&1), "plain click selects");

        router.route(
            &mut cx,
            &layout,
            pointer(PointerPhase::Down, 50.0, row_y(3), ModifiersState::CONTROL),
        );
        let selected = state.selection().get();
        assert!(selected.contains(&1) && selected.contains(&3), "ctrl adds");

        router.route(
            &mut cx,
            &layout,
            pointer(PointerPhase::Down, 50.0, row_y(2), ModifiersState::SHIFT),
        );
        // Anchor is row 3 (last toggle); shift extends 3..=2 → {2, 3}.
        let selected = state.selection().get();
        assert!(selected.contains(&2) && selected.contains(&3) && !selected.contains(&1));
    });
}

#[test]
fn selected_row_paints_selection_bg() {
    sigil::reactive::Runtime::scope("table-view-selected-pixel", || {
        let state = demo_state();
        let mut cx = Cx::new_root();
        let root = cx.add(
            table(state.clone())
                .row_height(ROW_H)
                .header_height(HEADER_H)
                .viewport_width(WIDTH)
                .viewport_height(VIEW_H),
        );
        let _layout = compute(&mut cx, root);
        let selection_bg = Theme::default().accent_scale.step(5);
        assert!(
            !rect_colors(&cx, root).contains(&selection_bg),
            "nothing selected at rest",
        );
        state.select_only(1);
        assert!(
            rect_colors(&cx, root).contains(&selection_bg),
            "selected row paints the accent-5 selection slice; \
             colours: {:?}",
            rect_colors(&cx, root),
        );
        state.clear_selection();
        assert!(
            !rect_colors(&cx, root).contains(&selection_bg),
            "clearing the selection restores the rest paint",
        );
    });
}

#[test]
fn hover_row_paints_hover_slice() {
    sigil::reactive::Runtime::scope("table-view-hover-pixel", || {
        let state = demo_state();
        let mut cx = Cx::new_root();
        let root = cx.add(
            table(state)
                .row_height(ROW_H)
                .header_height(HEADER_H)
                .viewport_width(WIDTH)
                .viewport_height(VIEW_H),
        );
        let layout = compute(&mut cx, root);
        let mut router = EventRouter::new();
        let hover = Theme::default().surface_hover;
        assert_eq!(
            rect_colors(&cx, root)
                .iter()
                .filter(|c| **c == hover)
                .count(),
            0,
            "rest: header is surface, no hover tint",
        );
        router.route(
            &mut cx,
            &layout,
            pointer(
                PointerPhase::Move,
                50.0,
                HEADER_H + ROW_H / 2.0,
                ModifiersState::empty(),
            ),
        );
        assert_eq!(
            rect_colors(&cx, root)
                .iter()
                .filter(|c| **c == hover)
                .count(),
            1,
            "hovered row paints a surface-hover slice",
        );
        router.route(
            &mut cx,
            &layout,
            pointer(PointerPhase::Move, 500.0, 400.0, ModifiersState::empty()),
        );
        assert_eq!(
            rect_colors(&cx, root)
                .iter()
                .filter(|c| **c == hover)
                .count(),
            0,
            "pointer leave restores the rest paint",
        );
    });
}

#[test]
fn hundred_thousand_rows_window_and_sort_without_rebuild() {
    sigil::reactive::Runtime::scope("table-view-100k", || {
        let rows = (0..100_000_u32)
            .map(|i| item(&format!("r{:07}", 99_999 - i), i64::from(i)))
            .collect::<Vec<_>>();
        let state = TableState::new(
            rows,
            vec![
                TableColumn::new("name", "Name")
                    .sort_by(|row: &Item| row.name.clone())
                    .width(ColWidth::Fixed(WIDTH))
                    .cell(|row: &Item| container().child(text(row.name.clone()))),
            ],
        );
        let mut cx = Cx::new_root();
        let root = cx.add(
            table(state.clone())
                .row_height(ROW_H)
                .header_height(HEADER_H)
                .viewport_width(WIDTH)
                .viewport_height(VIEW_H),
        );
        let _layout = compute(&mut cx, root);
        // 200px viewport / 20px rows → 11 materialized rows, not 100k.
        let body_runs = |cx: &Cx| {
            glyph_texts(cx, root)
                .into_iter()
                .filter(|text| text.starts_with('r'))
                .collect::<Vec<_>>()
        };
        assert_eq!(body_runs(&cx).len(), 11, "window stays viewport-sized");
        assert_eq!(body_runs(&cx).first().map(String::as_str), Some("r0099999"));

        // Sorting re-orders through the derived visible list — the
        // window rebuilds in place at the same 11-row cost.
        state.toggle_sort("name");
        let _layout = compute(&mut cx, root);
        assert_eq!(body_runs(&cx).len(), 11, "window cost unchanged by sort");
        assert_eq!(body_runs(&cx).first().map(String::as_str), Some("r0000000"));
    });
}

#[test]
fn quick_filter_keeps_keyboard_row_count_live() {
    sigil::reactive::Runtime::scope("table-view-live-count", || {
        let rows = (0..30_u32)
            .map(|i| {
                item(
                    &format!("{}-{i:02}", if i % 3 == 0 { "keep" } else { "drop" }),
                    i64::from(i),
                )
            })
            .collect::<Vec<_>>();
        let state = TableState::new(
            rows,
            vec![
                TableColumn::new("name", "Name")
                    .sort_by(|row: &Item| row.name.clone())
                    .filterable(|row: &Item, query: &str| row.name.contains(query))
                    .width(ColWidth::Fixed(WIDTH))
                    .cell(|row: &Item| container().child(text(row.name.clone()))),
            ],
        );
        let mut cx = Cx::new_root();
        let root = cx.add(
            table(state.clone())
                .row_height(ROW_H)
                .header_height(HEADER_H)
                .viewport_width(WIDTH)
                .viewport_height(100.0),
        );
        let _layout = compute(&mut cx, root);
        state.quick_filter().set("keep".to_owned());
        let layout = compute(&mut cx, root);
        assert_eq!(state.visible().get().len(), 10);

        // Focus the body list (second child of the grid root) and jump
        // to the end. A build-time snapshot (30) would clamp the window
        // past the 10 filtered rows and paint NOTHING; the live count
        // keeps End on the last filtered row.
        let body = cx
            .get(root)
            .and_then(|node| node.children().get(1).copied())
            .expect("grid root has a body child");
        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(body));
        router.route(&mut cx, &layout, key_down(Key::End, PhysicalKey::End));
        let _layout = compute(&mut cx, root);
        let texts = glyph_texts(&cx, root);
        assert!(
            texts.iter().any(|text| text == "keep-27"),
            "End must land on the last FILTERED row; painted: {texts:?}",
        );
    });
}

#[test]
fn wheel_scroll_advances_the_window() {
    sigil::reactive::Runtime::scope("table-view-wheel", || {
        let rows = (0..100_u32)
            .map(|i| item(&format!("r{i:03}"), i64::from(i)))
            .collect::<Vec<_>>();
        let state = TableState::new(
            rows,
            vec![
                TableColumn::new("name", "Name")
                    .sort_by(|row: &Item| row.name.clone())
                    .width(ColWidth::Fixed(WIDTH))
                    .cell(|row: &Item| container().child(text(row.name.clone()))),
            ],
        );
        let mut cx = Cx::new_root();
        let root = cx.add(
            table(state)
                .row_height(ROW_H)
                .header_height(HEADER_H)
                .viewport_width(WIDTH)
                .viewport_height(100.0),
        );
        let layout = compute(&mut cx, root);
        let first_body_text = |cx: &Cx| {
            glyph_texts(cx, root)
                .into_iter()
                .find(|text| text.starts_with('r'))
        };
        assert_eq!(first_body_text(&cx).as_deref(), Some("r000"));

        let mut router = EventRouter::new();
        router.route(
            &mut cx,
            &layout,
            ShellEvent::Scroll(ScrollEvent::new(ScrollEventInit {
                position: Point::new(50.0, 100.0),
                delta: ScrollDelta::Pixels(Point::new(0.0, -120.0)),
                modifiers: ModifiersState::empty(),
                phase: ScrollPhase::Moved,
            })),
        );
        let _layout = compute(&mut cx, root);
        // 120px down = six 20px rows.
        assert_eq!(
            first_body_text(&cx).as_deref(),
            Some("r006"),
            "wheel delta advances the visible window",
        );
    });
}

#[test]
fn empty_filter_shows_no_rows() {
    sigil::reactive::Runtime::scope("table-view-empty", || {
        let state = demo_state();
        state.quick_filter().set("zzzz".into());
        let mut cx = Cx::new_root();
        let root = cx.add(
            table(state)
                .row_height(ROW_H)
                .header_height(HEADER_H)
                .viewport_width(WIDTH)
                .viewport_height(VIEW_H),
        );
        let _ = compute(&mut cx, root);
        let texts = glyph_texts(&cx, root);
        assert!(
            texts.iter().any(|text| text.contains("No rows")),
            "empty table shows the default empty copy: {texts:?}"
        );
    });
}

#[test]
fn grid_a11y_channels_attach() {
    sigil::reactive::Runtime::scope("table-view-a11y", || {
        let state = demo_state();
        let mut cx = Cx::new_root();
        let root = cx.add(
            table(state)
                .row_height(ROW_H)
                .header_height(HEADER_H)
                .viewport_width(WIDTH)
                .viewport_height(VIEW_H),
        );
        let _layout = compute(&mut cx, root);
        let grid = cx.get(root).expect("root node");
        assert_eq!(grid.a11y.role, Role::Grid);
        assert_eq!(grid.a11y.column_count, Some(2));
        assert_eq!(
            grid.a11y.row_count.map(|count| count.get()),
            Some(5),
            "four body rows + one header row",
        );

        let header = grid.children().first().copied().expect("header row");
        let header = cx.get(header).expect("header node");
        assert_eq!(header.a11y.role, Role::Row);
        assert_eq!(header.a11y.row_index, Some(1));
        let header_cell = header.children().first().copied().expect("header cell");
        let header_cell = cx.get(header_cell).expect("header cell node");
        assert_eq!(header_cell.a11y.role, Role::ColumnHeader);
        assert_eq!(header_cell.a11y.column_index, Some(1));
        assert!(
            header_cell.a11y.sort_direction.is_some(),
            "sortable header carries the sort-direction channel",
        );
    });
}

const fn pointer_button(
    phase: PointerPhase,
    x: f32,
    y: f32,
    button: MouseButton,
    buttons: MouseButtons,
) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: Some(button),
        buttons,
        modifiers: ModifiersState::empty(),
        phase,
    }))
}

#[test]
fn resize_handle_pins_column_width() {
    sigil::reactive::Runtime::scope("table-view-resize", || {
        let state = demo_state();
        let mut cx = Cx::new_root();
        let root = cx.add(
            table(state.clone())
                .row_height(ROW_H)
                .header_height(HEADER_H)
                .viewport_width(WIDTH)
                .viewport_height(VIEW_H),
        );
        let layout = compute(&mut cx, root);
        let mut router = EventRouter::new();
        // Name column is Fixed(100); handle sits on the trailing 6 px.
        router.route(
            &mut cx,
            &layout,
            pointer_button(
                PointerPhase::Down,
                97.0,
                14.0,
                MouseButton::Left,
                MouseButtons::LEFT,
            ),
        );
        router.route(
            &mut cx,
            &layout,
            pointer_button(
                PointerPhase::Move,
                137.0,
                14.0,
                MouseButton::Left,
                MouseButtons::LEFT,
            ),
        );
        router.route(
            &mut cx,
            &layout,
            pointer_button(
                PointerPhase::Up,
                137.0,
                14.0,
                MouseButton::Left,
                MouseButtons::empty(),
            ),
        );
        assert_eq!(
            state.column_width("name"),
            Some(140.0),
            "drag +40 px pins the name column",
        );
    });
}

#[test]
fn right_click_row_selects() {
    sigil::reactive::Runtime::scope("table-view-context", || {
        let state = demo_state();
        let mut cx = Cx::new_root();
        let root = cx.add(
            table(state.clone())
                .row_height(ROW_H)
                .header_height(HEADER_H)
                .viewport_width(WIDTH)
                .viewport_height(VIEW_H),
        );
        let layout = compute(&mut cx, root);
        let mut router = EventRouter::new();
        let row_y = ROW_H.mul_add(1.5, HEADER_H);
        router.route(
            &mut cx,
            &layout,
            pointer_button(
                PointerPhase::Down,
                50.0,
                row_y,
                MouseButton::Right,
                MouseButtons::RIGHT,
            ),
        );
        assert!(
            state.selection().get().contains(&1),
            "right-click selects the row",
        );
    });
}

#[test]
fn fraction_column_honours_min_width() {
    sigil::reactive::Runtime::scope("table-min-width", || {
        let state = TableState::new(
            demo_rows(),
            vec![
                TableColumn::new("name", "Name")
                    .width(ColWidth::Fixed(300.0))
                    .cell(|row: &Item| container().child(text(row.name.clone()))),
                TableColumn::new("count", "Count")
                    .width(ColWidth::Fraction(1.0))
                    .min_width(192.0)
                    .cell(|row: &Item| container().child(text(row.count.to_string()))),
            ],
        );
        let mut cx = Cx::new_root();
        let root = cx.add(
            table(state)
                .row_height(ROW_H)
                .header_height(HEADER_H)
                .viewport_width(400.0)
                .viewport_height(VIEW_H),
        );
        let layout = compute(&mut cx, root);
        let header = *cx
            .get(root)
            .expect("grid")
            .children()
            .first()
            .expect("header");
        let count_cell = cx
            .get(header)
            .expect("header row")
            .children()
            .get(1)
            .copied()
            .expect("count header");
        let width = layout.result(count_cell).expect("count rect").rect().width;
        assert!(
            width >= 192.0,
            "Fraction min_width(192) must hold in a 400px table beside Fixed(300), got {width}"
        );
    });
}
