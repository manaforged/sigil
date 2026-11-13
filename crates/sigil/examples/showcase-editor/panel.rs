use sigil::prelude::*;
use sigil::style::FontFamily;

use super::facts::{DIAGNOSTICS, OUTPUT, PROMPT, Problem, severity_icon, severity_tone};
use super::state::{BottomTab, Workspace};

fn problem_row(ws: Workspace, diag: Problem) -> NodeBuilder {
    let location = format!("{}:{}:{}", diag.file.path(), diag.line + 1, diag.start + 1);
    list_row(diag.message)
        .leading(
            icon(severity_icon(diag.severity))
                .tone(severity_tone(diag.severity))
                .size(ControlSize::Compact),
        )
        .detail(format!("{location} [{}]", diag.code))
        .pointer_down(move |_| ws.goto(diag.file, diag.line, diag.start))
        .into()
}

fn fill_scroll(list: NodeBuilder) -> NodeBuilder {
    scroll(list).fill().into()
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
        column().fill_width().padding_y(Space::Xs).gap(Space::None),
        |list, diag| list.child(problem_row(ws, *diag)),
    );
    fill_scroll(list)
}

fn output() -> NodeBuilder {
    let list = OUTPUT.iter().fold(
        column()
            .fill_width()
            .padding_x(Space::Md)
            .padding_y(Space::Sm)
            .gap(Space::Xs),
        |list, line| {
            list.child(
                text(*line)
                    .nowrap()
                    .font_size(super::TEXT_XS, "example layout")
                    .font_family(FontFamily::Mono),
            )
        },
    );
    fill_scroll(list)
}

fn terminal_view(ws: Workspace) -> NodeBuilder {
    column()
        .fill()
        .min_width(0.0)
        .min_height(0.0)
        .padding_x(Space::Md)
        .padding_y(Space::Xs)
        .gap(Space::None)
        .child(terminal(ws.terminal, move |input| {
            ws.terminal.update(|lines| echo(lines, &input));
        }))
}

pub(crate) fn bottom_panel(ws: Workspace) -> NodeBuilder {
    let panel = BottomTab::ALL.into_iter().fold(tabs(), |panel, tab| {
        panel.tab(tab.label(), move || view(ws, tab))
    });
    column()
        .fill()
        .min_width(0.0)
        .min_height(0.0)
        .gap(Space::None)
        .background(Token::SurfaceSubtle)
        .radius(Radius::Md)
        .child(
            panel
                .active(ws.bottom)
                .label("Panel")
                .line()
                .justify_list(JustifyContent::Start),
        )
}

fn view(ws: Workspace, tab: BottomTab) -> NodeBuilder {
    match tab {
        BottomTab::Problems => problems(ws),
        BottomTab::Output => output(),
        BottomTab::Terminal => terminal_view(ws),
    }
}
