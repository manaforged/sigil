use sigil::prelude::*;
use sigil::style::space;

use super::facts::{DIAGNOSTICS, Diagnostic, OUTPUT, PROMPT, severity_glyph, severity_tone};
use super::state::{BottomTab, Workspace};

fn problem_row(ws: Workspace, diag: Diagnostic) -> NodeBuilder {
    let location = format!("{}:{}:{}", diag.file.path(), diag.line + 1, diag.start + 1);
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .padding_x(space(3))
                .padding_y(space(1))
                .gap(space(2))
                .align_items(AlignItems::Center),
        )
        .pointer_down(move |_| ws.goto(diag.file, diag.line, diag.start))
        .child(
            icon(severity_glyph(diag.severity))
                .tone(severity_tone(diag.severity))
                .size(ControlSize::Compact),
        )
        .child(text(diag.message).nowrap().font_size(super::TEXT_SM))
        .child(
            text(format!("{location} [{}]", diag.code))
                .ellipsize()
                .font_size(super::TEXT_XS)
                .text_color(Token::OnSurfaceMuted)
                .layout(LayoutStyle::block().grow(1.0).min_size(0.0, 0.0)),
        )
}

fn fill_scroll(list: NodeBuilder) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().fill().min_size(0.0, 0.0))
        .child(scroll(list).fill())
}

fn echo(lines: &mut Vec<String>, input: &str) {
    if input == "\n" {
        lines.push(String::from(PROMPT));
    } else if !input.chars().any(char::is_control)
        && let Some(last) = lines.last_mut()
    {
        last.push_str(input);
    }
}

fn problems(ws: Workspace) -> NodeBuilder {
    let list = DIAGNOSTICS.iter().fold(
        container().layout(LayoutStyle::flex_column().fill_width().padding_y(space(1))),
        |list, diag| list.child(problem_row(ws, *diag)),
    );
    fill_scroll(list)
}

fn output() -> NodeBuilder {
    let list = OUTPUT.iter().fold(
        container().layout(
            LayoutStyle::flex_column()
                .fill_width()
                .padding_x(space(3))
                .padding_y(space(2))
                .gap(space(1)),
        ),
        |list, line| {
            list.child(
                text(*line)
                    .nowrap()
                    .font_size(super::TEXT_XS)
                    .font_family(FontFamily::Mono),
            )
        },
    );
    fill_scroll(list)
}

fn terminal_view(ws: Workspace) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_column()
                .fill()
                .min_size(0.0, 0.0)
                .padding_x(space(3))
                .padding_y(space(1)),
        )
        .child(terminal(ws.terminal, move |input| {
            ws.terminal.update(|lines| echo(lines, &input));
        }))
}

fn panel_tabs(ws: Workspace) -> sigil::widgets::TabsBuilder {
    tabs()
        .labels(signal(BottomTab::ALL.map(BottomTab::label).to_vec()))
        .active(ws.bottom)
        .label("Panel")
        .line()
        .justify_list(JustifyContent::Start)
}

fn view(ws: Workspace, tab: BottomTab) -> NodeBuilder {
    match tab {
        BottomTab::Problems => problems(ws),
        BottomTab::Output => output(),
        BottomTab::Terminal => terminal_view(ws),
    }
}

pub(crate) fn bottom_panel(ws: Workspace) -> NodeBuilder {
    let current = derived(move || BottomTab::at(ws.bottom.get()));
    container()
        .layout(LayoutStyle::flex_column().fill().min_size(0.0, 0.0))
        .bg(Token::SurfaceSubtle)
        .rounded(Radius::Md)
        .child(panel_tabs(ws))
        .child(
            container()
                .layout(
                    LayoutStyle::flex_column()
                        .grow(1.0)
                        .fill_width()
                        .min_size(0.0, 0.0),
                )
                .children_reactive(move || [view(ws, current.get())]),
        )
}
