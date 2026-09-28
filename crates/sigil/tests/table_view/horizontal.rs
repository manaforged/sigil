use super::*;

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
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let mut router = EventRouter::new();

        let header_rest = header_cell_x(&cx, &layout, root, 1);
        let gap = text_x(&cx, root, "30").expect("gamma count cell at rest") - header_rest;
        let tag_rest = header_cell_x(&cx, &layout, root, 2);

        router.route(&mut cx, &layout, wheel_x_at(50.0, 14.0, -100.0));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
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
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
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
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
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
        let _layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
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
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        assert_eq!(scrollbar_track_count(&cx, root), 1, "overflow: track");
        let parked = thumb_x(&cx, root).expect("overflow: thumb");

        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, wheel_x_at(50.0, 100.0, -1_000.0));
        let _layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
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
        let mut layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
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
        let mut layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
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
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let tag_x = header_cell_x(&cx, &layout, grid, 2);
        let tag_cell = text_x(&cx, root, "tag-gamma").expect("tag cell revealed");
        assert!(
            (tag_cell - tag_x - gap).abs() < 0.01,
            "lockstep on the measured path: {tag_cell} vs {tag_x} + {gap}",
        );
    });
}
