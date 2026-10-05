use sigil::prelude::*;
use sigil::style::space;

use super::project::{FileId, FileKind, Folder, PROJECT_NAME};
use super::state::{Buffer, Workspace};

const PREVIEW_W: f32 = 340.0;
const PREVIEW_MIN: f32 = 200.0;
const SOURCE_MIN: f32 = 240.0;
const CRUMB_SEP: &str = "›";

fn tab_label(ws: Workspace, file: FileId) -> String {
    let name = file.spec().name;
    if ws.is_dirty(file) {
        format!("{name} ●")
    } else {
        String::from(name)
    }
}

fn tab_strip(ws: Workspace) -> sigil::widgets::TabsBuilder {
    let labels = derived(move || {
        ws.open
            .get()
            .into_iter()
            .map(|file| tab_label(ws, file))
            .collect::<Vec<String>>()
    });
    tabs()
        .labels(labels)
        .active(ws.active_tab)
        .label("Open editors")
        .closeable(true)
        .close(move |index| ws.close_tab(index))
        .line()
        .justify_list(JustifyContent::Start)
}

fn crumbs(file: FileId) -> Vec<String> {
    let spec = file.spec();
    let mut parts = vec![String::from(PROJECT_NAME)];
    parts.extend(
        Folder::chain(spec.folder)
            .into_iter()
            .map(|f| String::from(f.name())),
    );
    parts.push(String::from(spec.name));
    parts
}

fn breadcrumb(file: FileId) -> NodeBuilder {
    let parts = crumbs(file);
    let last = parts.len().saturating_sub(1);
    parts.into_iter().enumerate().fold(
        container().layout(
            LayoutStyle::flex_row()
                .fill_width()
                .shrink(0.0)
                .padding_x(space(3))
                .padding_y(space(1))
                .gap(space(2))
                .align_items(AlignItems::Center),
        ),
        |row, (index, part)| {
            let row = if index > 0 {
                row.child(
                    text(CRUMB_SEP)
                        .nowrap()
                        .font_size(super::TEXT_XS)
                        .text_color(Token::OnSurfaceMuted),
                )
            } else {
                row
            };
            let label = text(part).nowrap().font_size(super::TEXT_XS);
            row.child(if index == last {
                label.text_color(Token::OnSurface)
            } else {
                label.text_color(Token::OnSurfaceMuted)
            })
        },
    )
}

fn source(buffer: Buffer) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().fill().min_size(0.0, 0.0))
        .child(
            code_edit(buffer.lines, buffer.caret)
                .highlights(buffer.highlights)
                .gutter(buffer.gutter)
                .diagnostics(buffer.diagnostics),
        )
}

fn preview(buffer: Buffer) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_column()
                .fill()
                .min_size(0.0, 0.0)
                .padding(space(4)),
        )
        .bg(Token::SurfaceSubtle)
        .rounded(Radius::Md)
        .child(scroll(markdown(buffer.text)).fill())
}

fn body(ws: Workspace, file: FileId) -> NodeBuilder {
    let buffer = ws.buffer(file);
    let content: NodeBuilder = match file.spec().kind {
        FileKind::Markdown => splitter(Orientation::Horizontal)
            .pane(source(buffer))
            .min(SOURCE_MIN)
            .grow(1.0)
            .commit()
            .child_constrained(preview(buffer), PREVIEW_W, PREVIEW_MIN, PREVIEW_W * 2.0)
            .grow(1.0)
            .into(),
        FileKind::Rust | FileKind::Toml => source(buffer),
    };
    container()
        .layout(LayoutStyle::flex_column().fill().min_size(0.0, 0.0))
        .child(breadcrumb(file))
        .child(
            container()
                .layout(
                    LayoutStyle::flex_column()
                        .grow(1.0)
                        .fill_width()
                        .min_size(0.0, 0.0),
                )
                .child(content),
        )
}

fn no_editor() -> NodeBuilder {
    empty_state(
        "No open editors",
        "Open a file from the explorer or press Ctrl+P.",
    )
}

pub(crate) fn editor(ws: Workspace) -> NodeBuilder {
    let active = derived(move || ws.active());
    container()
        .layout(LayoutStyle::flex_column().fill().min_size(0.0, 0.0))
        .bg(Token::Surface)
        .child(tab_strip(ws))
        .child(
            container()
                .layout(
                    LayoutStyle::flex_column()
                        .grow(1.0)
                        .fill_width()
                        .min_size(0.0, 0.0),
                )
                .children_reactive(move || {
                    [active.get().map_or_else(no_editor, |file| body(ws, file))]
                }),
        )
}
