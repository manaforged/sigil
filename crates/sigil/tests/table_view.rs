#![allow(missing_docs)]
#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::too_many_lines)]
#![allow(clippy::shadow_unrelated)]

use sigil::hal::{KeyEventInit, PhysicalKey, Point, PointerEventInit, ScrollEventInit};
use sigil::prelude::*;
use sigil::renderer::{SceneCommand, emit_scene};
use sigil::widgets::{SortDir, SortKey, TableState};

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
    fn measure(&self, _content: &TextContent, _space: sigil::hal::Size) -> sigil::hal::LogicalSize {
        sigil::hal::LogicalSize::new(40.0, 16.0)
    }
}

fn compute(cx: &mut Cx, root: NodeId) -> LayoutEngine {
    let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
    layout
        .compute(cx, root, Size::fixed(640.0, 480.0))
        .expect("layout");
    layout
}

fn rect_colors(cx: &Cx, root: NodeId) -> Vec<sigil::style::Rgba8> {
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
        let body_runs = |cx: &Cx| {
            glyph_texts(cx, root)
                .into_iter()
                .filter(|text| text.starts_with('r'))
                .collect::<Vec<_>>()
        };
        assert_eq!(body_runs(&cx).len(), 11, "window stays viewport-sized");
        assert_eq!(body_runs(&cx).first().map(String::as_str), Some("r0099999"));

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
        let texts = glyph_texts(&cx, root);
        let body = texts
            .iter()
            .filter(|text| text.starts_with('r'))
            .collect::<Vec<_>>();
        assert_eq!(
            body.first().map(|s| s.as_str()),
            Some("r005"),
            "wheel delta advances the visible window: {texts:?}",
        );
        assert_eq!(
            body.get(1).map(|s| s.as_str()),
            Some("r006"),
            "first fully visible row is six rows down: {texts:?}",
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
        let track = header
            .children()
            .first()
            .copied()
            .expect("header cell track");
        let header_cell = cx
            .get(track)
            .expect("track node")
            .children()
            .first()
            .copied()
            .expect("header cell");
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
        router.route(
            &mut cx,
            &layout,
            pointer_button(
                PointerPhase::Down,
                109.0,
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
                149.0,
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
                149.0,
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
        let track = *cx
            .get(header)
            .expect("header row")
            .children()
            .first()
            .expect("header cell track");
        let count_cell = cx
            .get(track)
            .expect("track")
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

fn wide_state() -> TableState<Item> {
    TableState::new(
        demo_rows(),
        vec![
            TableColumn::new("name", "Name")
                .width(ColWidth::Fixed(150.0))
                .cell(|row: &Item| container().child(text(row.name.clone()))),
            TableColumn::new("count", "Count")
                .width(ColWidth::Fixed(150.0))
                .cell(|row: &Item| container().child(text(row.count.to_string()))),
            TableColumn::new("tag", "Tag")
                .width(ColWidth::Fixed(150.0))
                .cell(|row: &Item| container().child(text(format!("tag-{}", row.name)))),
        ],
    )
}

fn wide_table(state: TableState<Item>) -> impl IntoNode {
    table(state)
        .row_height(ROW_H)
        .header_height(HEADER_H)
        .viewport_width(WIDTH)
        .viewport_height(VIEW_H)
}

const fn wheel_x_at(x: f32, y: f32, dx: f32) -> ShellEvent {
    ShellEvent::Scroll(ScrollEvent::new(ScrollEventInit {
        position: Point::new(x, y),
        delta: ScrollDelta::Pixels(Point::new(dx, 0.0)),
        modifiers: ModifiersState::empty(),
        phase: ScrollPhase::Moved,
    }))
}

fn text_x(cx: &Cx, root: NodeId, expected: &str) -> Option<f32> {
    emit_scene(cx, root)
        .find_text(expected)
        .map(|draw| draw.run.rect.origin.x)
}

fn scrollbar_track_count(cx: &Cx, root: NodeId) -> usize {
    let host = cx
        .get(root)
        .and_then(|node| node.children().get(3).copied())
        .expect("grid root has a scrollbar host child");
    cx.get(host).map_or(0, |node| node.children().len())
}

fn header_cell_x(cx: &Cx, layout: &LayoutEngine, grid: NodeId, index: usize) -> f32 {
    let header = cx
        .get(grid)
        .and_then(|node| node.children().first().copied())
        .expect("grid root has a header row");
    let track = cx
        .get(header)
        .and_then(|node| node.children().first().copied())
        .expect("header row has a cell track");
    let cell = cx
        .get(track)
        .and_then(|node| node.children().get(index).copied())
        .expect("header cell");
    layout
        .result(cell)
        .expect("header cell rect")
        .rect()
        .origin
        .x
}

fn thumb_x(cx: &Cx, root: NodeId) -> Option<f32> {
    let thumb = Theme::default().surface_pressed_strong;
    emit_scene(cx, root).commands.iter().find_map(|c| match c {
        SceneCommand::Rect(r)
            if r.color == thumb && (r.rect.height - 8.0).abs() < 0.01 && r.rect.width >= 24.0 =>
        {
            Some(r.rect.origin.x)
        }
        _ => None,
    })
}

#[test]
fn horizontal_wheel_shifts_header_and_body_in_lockstep() {
    sigil::reactive::Runtime::scope("table-view-hscroll", || {
        let state = wide_state();
        let mut cx = Cx::new_root();
        let root = cx.add(wide_table(state));
        let layout = compute(&mut cx, root);
        let mut router = EventRouter::new();

        let header_rest = header_cell_x(&cx, &layout, root, 1);
        let gap = text_x(&cx, root, "30").expect("gamma count cell at rest") - header_rest;
        let tag_rest = header_cell_x(&cx, &layout, root, 2);

        router.route(&mut cx, &layout, wheel_x_at(50.0, 14.0, -100.0));
        let layout = compute(&mut cx, root);
        let header_x = header_cell_x(&cx, &layout, root, 1);
        assert!(
            (header_x - (header_rest - 100.0)).abs() < 0.01,
            "header cell shifted by the wheel delta: {header_rest} -> {header_x}",
        );
        let body_x = text_x(&cx, root, "30").expect("count cell after header scroll");
        assert!(
            (body_x - header_x - gap).abs() < 0.01,
            "body tracks the header: rest gap {gap}, now {}",
            body_x - header_x,
        );

        router.route(&mut cx, &layout, wheel_x_at(50.0, 100.0, -1_000.0));
        let layout = compute(&mut cx, root);
        let tag_x = header_cell_x(&cx, &layout, root, 2);
        assert!(
            (tag_x - (tag_rest - 250.0)).abs() < 0.01,
            "clamped at extent - viewport (250), not 100 + 1000: {tag_rest} -> {tag_x}",
        );
        let tag_cell = text_x(&cx, root, "tag-gamma").expect("tag cell revealed at the clamp");
        assert!(
            (tag_cell - tag_x - gap).abs() < 0.01,
            "body and header locked at the clamp: {tag_cell} vs {tag_x} + {gap}",
        );

        router.route(&mut cx, &layout, wheel_x_at(50.0, 100.0, -1_000.0));
        let layout = compute(&mut cx, root);
        assert!(
            (header_cell_x(&cx, &layout, root, 2) - tag_x).abs() < 0.01,
            "no movement past the clamp",
        );
    });
}

#[test]
fn fitting_table_has_no_scrollbar_and_no_offset() {
    sigil::reactive::Runtime::scope("table-view-hscroll-fits", || {
        let state = demo_state();
        let mut cx = Cx::new_root();
        let root = cx.add(wide_table(state));
        let _layout = compute(&mut cx, root);
        assert_eq!(
            scrollbar_track_count(&cx, root),
            0,
            "content fits: no track rendered",
        );
        assert_eq!(thumb_x(&cx, root), None, "content fits: no thumb");
    });
}

#[test]
fn over_wide_columns_reveal_a_sliding_thumb() {
    sigil::reactive::Runtime::scope("table-view-hscroll-thumb", || {
        let state = wide_state();
        let mut cx = Cx::new_root();
        let root = cx.add(wide_table(state));
        let layout = compute(&mut cx, root);
        assert_eq!(scrollbar_track_count(&cx, root), 1, "overflow: track");
        let parked = thumb_x(&cx, root).expect("overflow: thumb");

        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, wheel_x_at(50.0, 100.0, -1_000.0));
        let _layout = compute(&mut cx, root);
        let slid = thumb_x(&cx, root).expect("thumb after scroll");
        assert!(
            (slid - parked - 111.1).abs() < 0.5,
            "thumb slides to the right end at max scroll: {parked} -> {slid}",
        );
    });
}

fn body_cell_x(cx: &Cx, layout: &LayoutEngine, grid: NodeId, index: usize) -> f32 {
    fn find_row(cx: &Cx, id: NodeId) -> Option<NodeId> {
        let node = cx.get(id)?;
        if node.a11y.role == Role::Row && node.a11y.row_index == Some(2) {
            return Some(id);
        }
        node.children().iter().find_map(|kid| find_row(cx, *kid))
    }
    let body = cx
        .get(grid)
        .and_then(|node| node.children().get(1).copied())
        .expect("grid root has a body child");
    let row = find_row(cx, body).expect("a materialized row");
    let track = cx
        .get(row)
        .and_then(|node| node.children().first().copied())
        .expect("row has a cell track");
    let cell = cx
        .get(track)
        .and_then(|node| node.children().get(index).copied())
        .expect("body cell");
    layout.result(cell).expect("body cell rect").rect().origin.x
}

#[test]
fn fraction_columns_stretch_in_a_measured_viewport() {
    sigil::reactive::Runtime::scope("table-view-fraction-stretch", || {
        let state = TableState::new(
            demo_rows(),
            vec![
                TableColumn::new("name", "Name")
                    .width(ColWidth::Fraction(1.4))
                    .cell(|row: &Item| container().child(text(row.name.clone()))),
                TableColumn::new("count", "Count")
                    .width(ColWidth::Fraction(0.8))
                    .cell(|row: &Item| container().child(text(row.count.to_string()))),
                TableColumn::new("tag", "Tag")
                    .width(ColWidth::Fixed(96.0))
                    .cell(|row: &Item| container().child(text(format!("tag-{}", row.name)))),
            ],
        );
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(
                    LayoutStyle::flex_column()
                        .fixed_size(800.0, 400.0)
                        .padding(10.0),
                )
                .child(
                    table(state)
                        .row_height(ROW_H)
                        .header_height(HEADER_H)
                        .viewport_fill(),
                ),
        );
        let mut layout = compute(&mut cx, root);
        for _ in 0..3 {
            sigil::app::__drive_test_frame(
                || {
                    layout
                        .compute(&mut cx, root, Size::fixed(800.0, 400.0))
                        .expect("layout");
                },
                || {},
            );
        }
        let grid = cx
            .get(root)
            .and_then(|node| node.children().first().copied())
            .expect("wrapper holds the grid");
        let name_w = body_cell_x(&cx, &layout, grid, 1) - body_cell_x(&cx, &layout, grid, 0);
        assert!(
            (name_w - 418.7).abs() < 1.0,
            "Fraction(1.4) must take its content-independent share of the leftover, not sit at its 48px floor: {name_w}",
        );
        assert_eq!(
            scrollbar_track_count(&cx, grid),
            0,
            "fits: no horizontal scrollbar",
        );
    });
}

#[test]
fn unset_viewport_width_measures_the_table() {
    sigil::reactive::Runtime::scope("table-view-hscroll-measured", || {
        let state = wide_state();
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(WIDTH, 300.0))
                .child(
                    table(state)
                        .row_height(ROW_H)
                        .header_height(HEADER_H)
                        .viewport_height(VIEW_H),
                ),
        );
        let mut layout = compute(&mut cx, root);
        for _ in 0..3 {
            sigil::app::__drive_test_frame(
                || {
                    layout
                        .compute(&mut cx, root, Size::fixed(640.0, 480.0))
                        .expect("layout");
                },
                || {},
            );
        }
        let grid = cx
            .get(root)
            .and_then(|node| node.children().first().copied())
            .expect("wrapper holds the grid");
        assert_eq!(
            scrollbar_track_count(&cx, grid),
            1,
            "450 px of columns overflow the measured ~198 px content box",
        );

        let gap = text_x(&cx, root, "30").expect("gamma count cell at rest")
            - header_cell_x(&cx, &layout, grid, 1);
        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, wheel_x_at(50.0, 100.0, -1_000.0));
        let layout = compute(&mut cx, root);
        let tag_x = header_cell_x(&cx, &layout, grid, 2);
        let tag_cell = text_x(&cx, root, "tag-gamma").expect("tag cell revealed");
        assert!(
            (tag_cell - tag_x - gap).abs() < 0.01,
            "lockstep on the measured path: {tag_cell} vs {tag_x} + {gap}",
        );
    });
}

fn check_table(state: TableState<Item>) -> impl IntoNode {
    table(state)
        .selectable_checkboxes()
        .row_height(ROW_H)
        .header_height(HEADER_H)
        .viewport_width(WIDTH)
        .viewport_height(VIEW_H)
}

fn checkbox_nodes(cx: &Cx, root: NodeId) -> Vec<NodeId> {
    fn walk(cx: &Cx, id: NodeId, out: &mut Vec<NodeId>) {
        let Some(node) = cx.get(id) else {
            return;
        };
        if node.a11y.role == Role::CheckBox {
            out.push(id);
        }
        for child in node.children() {
            walk(cx, *child, out);
        }
    }
    let mut out = Vec::new();
    walk(cx, root, &mut out);
    out
}

fn header_checked(cx: &Cx, root: NodeId) -> Option<bool> {
    let header = checkbox_nodes(cx, root)
        .first()
        .copied()
        .expect("header box");
    cx.get(header)
        .and_then(|node| node.a11y.checked)
        .map(|checked| checked.get())
}

const CHECK_X: f32 = 28.0;

#[test]
fn checkbox_column_absent_by_default() {
    sigil::reactive::Runtime::scope("table-view-checks-default-off", || {
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
        assert!(
            checkbox_nodes(&cx, root).is_empty(),
            "no checkbox nodes without the opt-in",
        );
        assert_eq!(
            cx.get(root).expect("grid").a11y.column_count,
            Some(2),
            "column count unchanged without the opt-in",
        );
    });
}

#[test]
fn row_checkbox_toggles_exactly_once() {
    sigil::reactive::Runtime::scope("table-view-checks-toggle", || {
        let state = demo_state();
        let mut cx = Cx::new_root();
        let root = cx.add(check_table(state.clone()));
        let layout = compute(&mut cx, root);
        let mut router = EventRouter::new();
        assert_eq!(checkbox_nodes(&cx, root).len(), 5, "header + 4 row boxes");
        assert_eq!(
            cx.get(root).expect("grid").a11y.column_count,
            Some(3),
            "checkbox column counts into the grid",
        );

        let row_y = ROW_H.mul_add(1.5, HEADER_H);
        click(&mut router, &mut cx, &layout, CHECK_X, row_y);
        let selected = state.selection().get();
        assert!(
            selected.len() == 1 && selected.contains(&1),
            "one click checks the row exactly once (a leaked row pointer-down \
             would select-then-toggle back to empty): {selected:?}",
        );

        click(&mut router, &mut cx, &layout, CHECK_X, row_y);
        assert!(
            state.selection().get().is_empty(),
            "second click unchecks the row",
        );

        click(
            &mut router,
            &mut cx,
            &layout,
            100.0,
            ROW_H.mul_add(0.5, HEADER_H),
        );
        let selected = state.selection().get();
        assert!(
            selected.len() == 1 && selected.contains(&0),
            "row click still selects only that row: {selected:?}",
        );
    });
}

#[test]
fn header_checkbox_selects_all_visible_then_clears() {
    sigil::reactive::Runtime::scope("table-view-checks-all", || {
        let state = demo_state();
        let mut cx = Cx::new_root();
        let root = cx.add(check_table(state.clone()));
        let layout = compute(&mut cx, root);
        let mut router = EventRouter::new();

        click(&mut router, &mut cx, &layout, CHECK_X, HEADER_H / 2.0);
        let selected = state.selection().get();
        assert_eq!(selected.len(), 4, "header check selects every visible row");
        assert_eq!(header_checked(&cx, root), Some(true));

        click(&mut router, &mut cx, &layout, CHECK_X, HEADER_H / 2.0);
        assert!(
            state.selection().get().is_empty(),
            "header click with all selected clears",
        );
        assert_eq!(header_checked(&cx, root), Some(false));
    });
}

#[test]
fn header_checkbox_indeterminate_when_partial() {
    sigil::reactive::Runtime::scope("table-view-checks-partial", || {
        let state = demo_state();
        let mut cx = Cx::new_root();
        let root = cx.add(check_table(state.clone()));
        let layout = compute(&mut cx, root);
        let mut router = EventRouter::new();

        state.select_only(1);
        assert_eq!(
            header_checked(&cx, root),
            Some(false),
            "partial selection does not read as fully checked",
        );

        click(&mut router, &mut cx, &layout, CHECK_X, HEADER_H / 2.0);
        assert_eq!(
            state.selection().get().len(),
            4,
            "click from partial selects all visible rows",
        );
        assert_eq!(header_checked(&cx, root), Some(true));
    });
}

#[test]
fn header_checkbox_selects_only_visible_rows() {
    sigil::reactive::Runtime::scope("table-view-checks-visible", || {
        let state = demo_state();
        state.quick_filter().set("ta".to_owned());
        let mut cx = Cx::new_root();
        let root = cx.add(check_table(state.clone()));
        let layout = compute(&mut cx, root);
        let mut router = EventRouter::new();

        click(&mut router, &mut cx, &layout, CHECK_X, HEADER_H / 2.0);
        let selected = state.selection().get();
        assert!(
            selected.len() == 2 && selected.contains(&2) && selected.contains(&3),
            "select-all covers visible rows only: {selected:?}",
        );
        assert_eq!(
            header_checked(&cx, root),
            Some(true),
            "every visible row selected reads checked",
        );
    });
}
