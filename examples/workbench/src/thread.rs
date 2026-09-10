use sigil::prelude::*;

use crate::icons;
use crate::state::{Line, Page, Shell, Speaker};

pub(crate) fn main_col(shell: Shell) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_column()
                .grow(1.0)
                .fill_height()
                .min_size(0.0, 0.0),
        )
        .class(classes!("bg-gray-1"))
        .children_keyed(move || match shell.page.get() {
            Page::Activity => vec![(0_u8, node(thread_stage(shell)))],
            Page::Schedules => vec![(1, node(crate::pages::schedules(shell)))],
            Page::Tools => vec![(2, node(crate::pages::tools()))],
        })
}

fn thread_stage(shell: Shell) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_row().fill())
        .child(thread_page(shell))
        .child(diff_slot(shell))
}

fn diff_slot(shell: Shell) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_row().fill_height().shrink(0.0))
        .children_reactive(move || {
            if shell.show_diff.get() {
                vec![node(crate::diffs::pane(shell))]
            } else {
                Vec::new()
            }
        })
}

fn thread_page(shell: Shell) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_column()
                .grow(1.0)
                .fill_height()
                .min_size(0.0, 0.0),
        )
        .child(title_bar(shell))
        .child(body(shell))
        .child(crate::compose::composer(shell))
        .child(crate::compose::status_strip(shell))
}

fn body(shell: Shell) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_column()
                .fill_width()
                .grow(1.0)
                .min_size(0.0, 0.0),
        )
        .children_keyed(move || {
            let id = shell.selected.get();
            if id == 0 {
                vec![(0_u32, node(crate::pages::empty_body(shell)))]
            } else {
                vec![(id, node(transcript(shell)))]
            }
        })
}

fn title_bar(shell: Shell) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .height(52.0)
                .padding_x(20.0)
                .gap(8.0)
                .align_items(AlignItems::Center),
        )
        .child(title_labels(shell))
        .child(spacer())
        .child(icon_hit(icons::play(), "Run"))
        .child(split_btn("Open"))
        .child(split_btn("Commit"))
        .child(hairline())
        .child(diff_badge(shell))
}

fn title_labels(shell: Shell) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .gap(8.0)
                .align_items(AlignItems::Center)
                .grow(1.0)
                .min_size(0.0, 16.0),
        )
        .children_reactive(move || {
            let Some(thread) = shell.current() else {
                return vec![
                    node(label("New note").class(classes!("text-sm font-medium text-gray-12"))),
                    node(label("New project").class(classes!("text-sm text-gray-11"))),
                ];
            };
            vec![
                node(
                    label(thread.title)
                        .ellipsize()
                        .class(classes!("text-sm font-medium text-gray-12")),
                ),
                node(label(thread.project).class(classes!("text-sm text-gray-11"))),
                node(label("···").class(classes!("text-sm text-gray-11"))),
            ]
        })
}

fn split_btn(title: &'static str) -> NodeBuilder {
    container()
        .role(Role::Button)
        .label(title)
        .focusable(true)
        .layout(
            LayoutStyle::flex_row()
                .height(28.0)
                .padding_x(10.0)
                .gap(4.0)
                .align_items(AlignItems::Center),
        )
        .class(classes!("rounded-md border border-gray-6"))
        .child(label(title).class(classes!("text-xs text-gray-12")))
        .child(icons::chevron_down())
}

fn icon_hit(glyph: PathNode, name: &'static str) -> NodeBuilder {
    container()
        .role(Role::Button)
        .label(name)
        .focusable(true)
        .layout(
            LayoutStyle::flex_row()
                .fixed_size(28.0, 28.0)
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::Center),
        )
        .class(classes!("rounded-md"))
        .child(glyph)
}

fn diff_badge(shell: Shell) -> NodeBuilder {
    let on = derived(move || shell.show_diff.get());
    container()
        .role(Role::Button)
        .label("Uncommitted changes")
        .focusable(true)
        .layout(
            LayoutStyle::flex_row()
                .height(28.0)
                .padding_x(8.0)
                .gap(6.0)
                .align_items(AlignItems::Center),
        )
        .class(classes!("rounded-md border border-gray-6"))
        .class_when(on, "bg-gray-4")
        .pointer_down(move |event| {
            shell.toggle_diff();
            event.stop_propagation();
        })
        .child(icons::diffs())
        .child(label("+129").class(classes!("text-xs text-success-11")))
        .child(label("-16").class(classes!("text-xs text-error-11")))
}

fn transcript(shell: Shell) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_column()
                .fill_width()
                .grow(1.0)
                .min_size(0.0, 0.0)
                .padding_x(36.0)
                .padding_y(20.0)
                .gap(20.0)
                .overflow_y(Overflow::Scroll),
        )
        .children_keyed(move || {
            let Some(thread) = shell.current() else {
                return Vec::new();
            };
            thread
                .lines
                .into_iter()
                .enumerate()
                .map(|(i, line)| (u32::try_from(i).unwrap_or(u32::MAX), line_row(line)))
                .collect::<Vec<_>>()
        })
}

fn line_row(line: Line) -> NodeBuilder {
    match line.speaker {
        Speaker::User => container()
            .layout(
                LayoutStyle::flex_row()
                    .fill_width()
                    .justify_content(JustifyContent::End),
            )
            .child(
                container()
                    .layout(
                        LayoutStyle::flex_row()
                            .padding_x(14.0)
                            .padding_y(8.0)
                            .max_size(520.0, f32::INFINITY),
                    )
                    .class(classes!("rounded-full bg-gray-4"))
                    .child(text(line.body).class(classes!("text-sm text-gray-12"))),
            ),
        Speaker::System => {
            let mut col = container().layout(LayoutStyle::flex_column().fill_width().gap(8.0));
            for run in inline_runs(&line.body) {
                col = col.child(run);
            }
            col
        }
    }
}

fn inline_runs(body: &str) -> Vec<NodeBuilder> {
    body.split('\n')
        .map(|para| {
            para.strip_prefix("• ")
                .map_or_else(|| rich_line(para), bullet_row)
        })
        .collect()
}

fn bullet_row(rest: &str) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .gap(8.0)
                .align_items(AlignItems::Center),
        )
        .child(label("•").class(classes!("text-sm text-gray-12")))
        .child(rich_line(rest))
}

fn rich_line(body: &str) -> NodeBuilder {
    let mut row = wrap_row();
    let mut rest = body;
    let mut pieces = 0_u8;
    while let Some(start) = rest.find('`') {
        if start > 0 {
            row = row.child(plain_run(&rest[..start]));
            pieces = pieces.saturating_add(1);
        }
        rest = &rest[start + 1..];
        if let Some(end) = rest.find('`') {
            row = row.child(badge(rest[..end].to_owned(), Tone::Neutral));
            pieces = pieces.saturating_add(1);
            rest = &rest[end + 1..];
        } else {
            break;
        }
    }
    if !rest.is_empty() {
        if pieces == 0 {
            return plain_run(rest);
        }
        row = row.child(plain_run(rest));
    }
    row
}

fn wrap_row() -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .gap(4.0)
                .align_items(AlignItems::Center)
                .min_size(0.0, 0.0),
        )
        .class(classes!("flex-wrap"))
}

fn plain_run(body: &str) -> NodeBuilder {
    let mut parts: Vec<NodeBuilder> = Vec::new();
    let mut buf = String::new();
    for word in body.split_whitespace() {
        if is_link(word) {
            if !buf.is_empty() {
                parts.push(chunk(prose(std::mem::take(&mut buf))));
            }
            parts.push(chunk(
                text(word.to_owned()).class(classes!("text-sm text-info-11")),
            ));
        } else {
            if !buf.is_empty() {
                buf.push(' ');
            }
            buf.push_str(word);
        }
    }
    if !buf.is_empty() {
        parts.push(chunk(prose(buf)));
    }
    match parts.len() {
        0 => chunk(prose(String::new())),
        1 => parts.remove(0),
        _ => {
            let mut row = wrap_row();
            for part in parts {
                row = row.child(part);
            }
            row
        }
    }
}

fn prose(body: String) -> impl IntoNode + 'static {
    text(body).class(classes!("text-sm text-gray-12"))
}

fn chunk(child: impl IntoNode + 'static) -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().min_size(0.0, 0.0))
        .child(child)
}

fn is_link(word: &str) -> bool {
    std::path::Path::new(word)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            ext.eq_ignore_ascii_case("md")
                || ext.eq_ignore_ascii_case("ts")
                || ext.eq_ignore_ascii_case("rs")
        })
}

fn hairline() -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().fixed_size(1.0, 18.0))
        .class(classes!("bg-gray-6"))
}

fn spacer() -> NodeBuilder {
    container().layout(LayoutStyle::block().grow(1.0))
}
