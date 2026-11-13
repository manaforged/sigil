#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;

const PAGES: &[&str] = &[
    "Controls",
    "Inputs",
    "Data",
    "Content",
    "Documents",
    "Chrome",
    "Overlays",
];

const PATCH: &str = "--- a/src/app.rs\n+++ b/src/app.rs\n@@ -1,3 +1,3 @@\n fn main() {\n-    old();\n+    fresh();\n }";

fn main() -> sigil::AppResult<()> {
    sigil::run(
        AppConfig::new("Sigil - widget gallery").size(1040, 680),
        build,
    )
}

fn build() -> impl IntoNode {
    let page = signal(0_usize);
    let dialog = signal(false);
    let drawer = signal(false);
    let queue = ToastQueue::new();
    row()
        .role(Role::Main)
        .fill()
        .gap(Space::None)
        .align_items(AlignItems::Stretch)
        .background(Token::Surface)
        .child(sidebar(nav(page)))
        .child(content(page, dialog, drawer, queue))
        .child(
            modal(dialog)
                .title("Confirm action")
                .child(text("This modal is mounted at the root.").nowrap())
                .child(
                    button("Close")
                        .primary()
                        .activate(move || dialog.set(false)),
                ),
        )
        .child(
            modal(drawer)
                .side(Side::Right)
                .dismiss_on_backdrop(true)
                .title("Details")
                .child(text("A right-edge sheet.").nowrap()),
        )
        .child(toast_queue(queue))
}

fn nav(page: Signal<usize>) -> NodeBuilder {
    let mut col = column().fill_height().padding(Space::Md).gap(Space::Xxs);
    col = col.child(section_label("Widgets"));
    for (index, name) in PAGES.iter().enumerate() {
        col = col.child(nav_row(page, index, name));
    }
    col
}

fn nav_row(page: Signal<usize>, index: usize, name: &'static str) -> NodeBuilder {
    let active = memo(move || page.get() == index);
    row()
        .role(Role::Button)
        .focusable(true)
        .fill_width()
        .height(28.0)
        .padding_x(Space::Sm)
        .align_items(AlignItems::Center)
        .gap(Space::None)
        .radius(Radius::Md)
        .paint_when(active, |s| s.background(Token::Accent4))
        .child(text(name).nowrap().text_role(TextRole::Body))
        .pointer_up(move |_| page.set(index))
}

fn content(
    page: Signal<usize>,
    dialog: Signal<bool>,
    drawer: Signal<bool>,
    queue: ToastQueue,
) -> impl IntoNode {
    scroll(
        column()
            .fill_width()
            .padding(Space::Xl)
            .gap(Space::raw(20.0, "example layout"))
            .children_reactive(move || {
                vec![match page.get() {
                    1 => inputs_page(),
                    2 => data_page(),
                    3 => content_page(),
                    4 => documents_page(),
                    5 => chrome_page(),
                    6 => overlays_page(dialog, drawer, queue),
                    _ => controls_page(),
                }]
            }),
    )
    .fill()
}

fn page_column() -> NodeBuilder {
    column().fill_width().gap(Space::Sm)
}

fn page_row() -> NodeBuilder {
    row().fill_width().gap(Space::Sm)
}

fn row2(a: impl IntoNode + 'static, b: impl IntoNode + 'static) -> NodeBuilder {
    page_row().child(a).child(b)
}

fn row3(
    a: impl IntoNode + 'static,
    b: impl IntoNode + 'static,
    c: impl IntoNode + 'static,
) -> NodeBuilder {
    page_row().child(a).child(b).child(c)
}

fn controls_page() -> NodeBuilder {
    let clicks = signal(0_i32);
    let mode = signal(String::from("auto"));
    let level = signal(40.0_f32);
    page_column()
        .child(section_label("Buttons"))
        .child(row3(
            button("Increment")
                .primary()
                .activate(move || clicks.set(clicks.peek() + 1)),
            button("⟲")
                .ghost()
                .square()
                .activate(move || clicks.set(0))
                .accessible("Reset clicks"),
            button("Disabled").disabled(true),
        ))
        .child(section_label("Toggles"))
        .child(row2(
            checkbox(signal(true)).label("Enabled"),
            toggle(signal(false)).label("Locked"),
        ))
        .child(field(
            "Radio group",
            radio_group(mode)
                .option("auto", "Automatic")
                .option("manual", "Manual"),
        ))
        .child(field("Slider", slider(level, 0.0..=100.0).step(5.0)))
        .child(field(
            "Gauge",
            gauge(level).max(100.0).label("Battery").warn(0.25),
        ))
        .child(field("Progress", progress(level, 100.0)))
}

fn inputs_page() -> NodeBuilder {
    let notes = signal(String::from("Multi-line\neditable text."));
    let combo = Combobox::new(vec![
        String::from("Rust"),
        String::from("Go"),
        String::from("Zig"),
        String::from("C++"),
    ])
    .placeholder("Language");
    page_column()
        .child(field(
            "Text field",
            text_field(signal(String::new())).placeholder("Full name"),
        ))
        .child(field("Text edit", text_edit(notes).height(96.0)))
        .child(field("Number", number_field(signal(4_i32), 1..=99)))
        .child(field(
            "Date",
            date_field(signal(Date::new(2026, 4, 1).expect("a calendar date"))),
        ))
        .child(field(
            "Dropdown",
            dropdown(
                signal(1_usize),
                vec!["Small".into(), "Medium".into(), "Large".into()],
            ),
        ))
        .child(field("Combobox", combobox(combo)))
}

struct DemoRow {
    name: &'static str,
    lang: &'static str,
    stars: u32,
}

const fn demo_row(name: &'static str, lang: &'static str, stars: u32) -> DemoRow {
    DemoRow { name, lang, stars }
}

fn demo_columns() -> Vec<TableColumn<DemoRow>> {
    vec![
        TableColumn::new("name", "Name")
            .sort_by(|row: &DemoRow| row.name)
            .filterable(|row, query| row.name.contains(query))
            .cell(|row| text(row.name).nowrap()),
        TableColumn::new("lang", "Language").cell(|row: &DemoRow| text(row.lang).nowrap()),
        TableColumn::new("stars", "Stars")
            .numeric()
            .sort_by(|row: &DemoRow| row.stars)
            .cell(|row| text(row.stars.to_string()).nowrap()),
    ]
}

fn data_page() -> NodeBuilder {
    let items: Vec<String> = (1..=40).map(|i| format!("Item {i}")).collect();
    page_column()
        .child(section_label("Table"))
        .child(
            table(TableState::new(
                vec![
                    demo_row("sigil", "Rust", 4210),
                    demo_row("wgpu", "Rust", 12800),
                    demo_row("zellij", "Rust", 14200),
                    demo_row("helix", "Rust", 36000),
                ],
                demo_columns(),
            ))
            .striped()
            .viewport_height(190.0),
        )
        .child(section_label("Virtual list"))
        .child(
            virtual_list(items, |index, item| {
                list_row(
                    false,
                    item.clone(),
                    ListRowSlots {
                        status: Some(format!("{index}").into()),
                        ..ListRowSlots::default()
                    },
                )
            })
            .selection(SelectionMode::Single)
            .viewport_height(140.0),
        )
}

fn content_page() -> NodeBuilder {
    page_column()
        .child(section_label("Text"))
        .child(text("Labels and text share TextContent.").nowrap())
        .child(section_label("Icons and indicators"))
        .child(row3(
            icon("★").tone(Tone::Warning).label("Star"),
            chip("stable", Tone::Success),
            chip("4", Tone::Error),
        ))
        .child(row3(
            dot(Tone::Success),
            dot(Tone::Warning),
            dot(Tone::Error),
        ))
        .child(section_label("Chart"))
        .child(
            chart([12.0, 28.0, 18.0, 34.0])
                .labels(["Q1", "Q2", "Q3", "Q4"])
                .height(160.0),
        )
        .child(section_label("Vector path"))
        .child(
            path(24, 24)
                .move_to(4.0, 20.0)
                .line_to(12.0, 4.0)
                .line_to(20.0, 20.0)
                .close()
                .fill(Color {
                    r: 90,
                    g: 140,
                    b: 255,
                    a: 255,
                })
                .fit(),
        )
}

fn documents_page() -> NodeBuilder {
    let source = signal(String::from(
        "# Sigil widgets\n\nEdit me — *live* preview.\n\n- one\n- two",
    ));
    page_column()
        .child(section_label("Markdown"))
        .child(
            column()
                .fill_width()
                .height(240.0)
                .gap(Space::None)
                .child(markdown_edit(source)),
        )
        .child(section_label("Diff"))
        .child(
            diff(PATCH)
                .path("src/app.rs")
                .view(DiffView::Unified)
                .max_rows(24),
        )
        .child(section_label("Mermaid"))
        .child(mermaid("graph TD\n  Edit --> Parse\n  Parse --> Paint"))
}

fn chrome_page() -> NodeBuilder {
    let tab = signal(0_usize);
    page_column()
        .child(section_label("Menu bar"))
        .child(menu_bar().fill_width().menu(
            "File",
            [
                MenuItem::new("New").activate(|| {}),
                MenuItem::new("Quit").hint("Ctrl+Q").activate(|| {}),
            ],
        ))
        .child(section_label("Panel and card"))
        .child(row2(
            card(text("Card body").nowrap()).title("Card"),
            card(kv_grid([("Host", "wgpu"), ("Backend", "metal")])),
        ))
        .child(section_label("Tabs"))
        .child(
            tabs()
                .active(tab)
                .tab("First", move || text("First tab pane").nowrap())
                .tab("Second", move || text("Second tab pane").nowrap()),
        )
        .child(section_label("Splitter"))
        .child(
            splitter(Orientation::Horizontal)
                .child(card(text("Navigation").nowrap()), 180.0)
                .child(card(text("Content").nowrap()), 280.0)
                .height(140.0),
        )
        .child(section_label("Patterns"))
        .child(notice_band("Deployment finished", Tone::Success))
        .child(row2(
            avatar("TG"),
            list_row(
                true,
                "Selected row",
                ListRowSlots {
                    status: Some("now".into()),
                    ..ListRowSlots::default()
                },
            ),
        ))
        .child(hint_bar("↑↓ navigate", "Enter select"))
}

fn overlays_page(dialog: Signal<bool>, drawer: Signal<bool>, queue: ToastQueue) -> NodeBuilder {
    page_column()
        .child(section_label("Dialog and sheet"))
        .child(row2(
            button("Open modal")
                .primary()
                .activate(move || dialog.set(true)),
            button("Open sheet").activate(move || drawer.set(true)),
        ))
        .child(section_label("Toasts"))
        .child(row3(
            button("Info").activate(move || queue.info("Heads up")),
            button("Success").activate(move || queue.success("Saved")),
            button("Error").activate(move || queue.error("Failed")),
        ))
        .child(section_label("Tooltip"))
        .child(tooltip("Saves the document").child(button("Hover me").ghost()))
        .child(section_label("Keyboard hints"))
        .child(hint_overlay(&[("Ctrl+K", "Palette"), ("Esc", "Close")]))
        .child(section_label("Command menu"))
        .child(
            command_menu()
                .item("Refresh", || {})
                .submenu("Export", |sub| sub.item("PNG", || {}).item("SVG", || {})),
        )
}
