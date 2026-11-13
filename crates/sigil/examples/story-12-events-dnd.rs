use sigil::events::{DropEvent, KeyEvent};
use sigil::prelude::*;
use sigil::style::Metrics;
use sigil::widgets::{DragState, drag_ghost};

fn main() -> sigil::AppResult<()> {
    sigil::run("Events & DnD", app)
}

#[derive(Clone)]
struct DndState {
    items: Signal<Vec<String>>,
    drag: DragState<String>,
}

/// Interactive Layer 1 and Layer 2 screen for `prevent_default`, `track_bounds`, and `DropEvent`.
#[must_use]
pub fn app() -> impl IntoNode {
    let buffer = signal(String::new());
    let parent_tabs = signal(0_u32);
    let anchor_bounds = signal(Rect::new(Point::new(0.0, 0.0), 0.0, 0.0));
    let menu_open = signal(false);
    let chosen_action = signal("none");
    let dnd = DndState {
        items: signal(vec![
            String::from("Alpha"),
            String::from("Beta"),
            String::from("Gamma"),
        ]),
        drag: DragState::new(),
    };

    column()
        .padding(Space::Md)
        .gap(Space::Sm)
        .background(Token::Surface)
        .child(text("Events & Drag-Drop").text_role(TextRole::Title))
        .child(tab_section(buffer, parent_tabs, anchor_bounds))
        .child(popup_controls(anchor_bounds, menu_open, chosen_action))
        .child(reorder_section(&dnd))
        .child(drag_ghost(dnd.drag, |item| {
            card(text(item.clone()).nowrap())
        }))
        .child(anchor_popup(menu_open, anchor_bounds, chosen_action))
}

fn row_pad_y() -> Space {
    if Metrics::current().is_grid() {
        Space::None
    } else {
        Space::Xs
    }
}

fn tab_section(
    buffer: Signal<String>,
    parent_tabs: Signal<u32>,
    anchor_bounds: Signal<Rect>,
) -> NodeBuilder {
    column()
        .gap(Space::Xs)
        .key(NamedKey::Tab, move |_| parent_tabs.update(|n| *n += 1))
        .child(
            container()
                .role(Role::TextInput)
                .label("Code buffer")
                .focusable(true)
                .autofocus()
                .padding_x(Space::Sm)
                .padding_y(row_pad_y())
                .radius(Radius::Md)
                .background(Token::SurfaceSubtle)
                .track_bounds(anchor_bounds)
                .key(NamedKey::Tab, move |event| {
                    event.prevent_default();
                    buffer.update(|s| s.push_str("  "));
                })
                .child(text(move || format!("Buffer: {:?}", buffer.get())).nowrap()),
        )
        .child(
            row()
                .gap(Space::Sm)
                .align_items(AlignItems::Center)
                .child(
                    button("Stop-only")
                        .size(ControlSize::Compact)
                        .key(NamedKey::Tab, KeyEvent::stop_propagation),
                )
                .child(button("After stop").size(ControlSize::Compact))
                .child(text(move || format!("Parent tabs: {}", parent_tabs.get())).nowrap()),
        )
}

fn popup_controls(
    anchor_bounds: Signal<Rect>,
    menu_open: Signal<bool>,
    chosen_action: Signal<&'static str>,
) -> NodeBuilder {
    row()
        .gap(Space::Sm)
        .align_items(AlignItems::Center)
        .child(
            button("Anchor menu")
                .size(ControlSize::Compact)
                .activate(move || menu_open.update(|open| *open = !*open)),
        )
        .child(text(move || format!("Menu: {}", chosen_action.get())).nowrap())
        .child(
            text(move || {
                let bounds = anchor_bounds.get();
                format!(
                    "Anchor: {:.0},{:.0}",
                    bounds.origin.x,
                    bounds.origin.y + bounds.height
                )
            })
            .nowrap(),
        )
}

fn anchor_popup(
    menu_open: Signal<bool>,
    anchor_bounds: Signal<Rect>,
    chosen_action: Signal<&'static str>,
) -> impl IntoNode {
    show(
        menu_open,
        move || {
            let bounds = anchor_bounds.peek();
            menu(menu_open)
                .label("Caret actions")
                .anchor(Point::new(bounds.origin.x, bounds.origin.y + bounds.height))
                .item(
                    MenuItem::new("Insert snippet")
                        .activate(move || chosen_action.set("Inserted snippet")),
                )
                .item(MenuItem::new("Format").activate(move || chosen_action.set("Formatted")))
        },
        container,
    )
}

fn reorder_section(dnd: &DndState) -> NodeBuilder {
    let dnd_rows = dnd.clone();
    let items = dnd.items;
    column()
        .gap(Space::Xs)
        .child(text(move || format!("Order: {}", items.get().join(" -> "))).nowrap())
        .child(column().fill_width().gap(Space::Xs).child(for_each(
            items,
            String::clone,
            move |label| reorder_row(label.clone(), &dnd_rows),
        )))
}

fn reorder_row(label: String, dnd: &DndState) -> NodeBuilder {
    let hover = signal(None::<Point>);
    let size = signal(Size::new(0.0, 0.0));
    let items = dnd.items;
    let drag = dnd.drag.clone();
    let accepts_label = label.clone();
    let drop_label = label.clone();
    let source_label = label.clone();
    let row_label = label.clone();
    let indicator = memo(move || match hover.get() {
        Some(pos) if pos.y < size.get().height * 0.5 => "▲ before",
        Some(_) => "▼ after",
        None => "",
    });
    let hovered = memo(move || hover.get().is_some());

    row()
        .role(Role::ListItem)
        .label(row_label)
        .fill_width()
        .padding_x(Space::Sm)
        .padding_y(row_pad_y())
        .gap(Space::Md)
        .radius(Radius::Md)
        .background(Token::SurfaceSubtle)
        .paint_when(hovered, |s| s.background(Token::SurfaceHover))
        .track_size(size)
        .drag_source(drag.clone(), source_label)
        .drop_target(
            drag.clone(),
            move |from: &String| from != &accepts_label,
            move |event: DropEvent<String>| {
                let after = event.local_position().y >= event.bounds().height * 0.5;
                let dragged = event.into_value();
                reorder_items(items, &dragged, &drop_label, after);
            },
        )
        .drop_hover(drag, hover)
        .child(text(label).nowrap().fill_width())
        .child(text(indicator).nowrap().text_color(Token::Accent))
}

fn reorder_items(items: Signal<Vec<String>>, dragged: &str, target: &str, after: bool) {
    items.update(|list| {
        let Some(from) = list.iter().position(|item| item == dragged) else {
            return;
        };
        let moving = list.remove(from);
        let Some(pos) = list.iter().position(|item| item == target) else {
            list.insert(from, moving);
            return;
        };
        let insert_at = if after { pos + 1 } else { pos };
        list.insert(insert_at, moving);
    });
}
