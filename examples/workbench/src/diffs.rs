use sigil::prelude::*;

use crate::state::Shell;

pub(crate) fn pane(shell: Shell) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_column()
                .width(320.0)
                .fill_height()
                .padding(16.0)
                .gap(10.0)
                .shrink(0.0),
        )
        .class(classes!("bg-gray-2 border-l border-gray-6"))
        .child(header())
        .child(file_chip())
        .child(hunk())
        .child(comment(shell))
        .child(hunk_tail())
}

fn header() -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .align_items(AlignItems::Center)
                .gap(8.0),
        )
        .child(label("Uncommitted changes").class(classes!("text-sm font-medium text-gray-12")))
        .child(spacer())
        .child(label("Unstaged · 304").class(classes!("text-xs text-gray-11")))
        .child(badge("Staged · 4", Tone::Neutral))
}

fn file_chip() -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .padding_y(4.0)
                .align_items(AlignItems::Center)
                .gap(8.0),
        )
        .child(
            label("src/tests/marks.test.ts")
                .ellipsize()
                .layout(LayoutStyle::block().grow(1.0).min_size(0.0, 14.0))
                .class(classes!("text-xs text-gray-12")),
        )
        .child(label("+119").class(classes!("text-xs text-success-11")))
        .child(label("-1").class(classes!("text-xs text-error-11")))
}

fn hunk() -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().fill_width().gap(1.0))
        .class(classes!(
            "rounded-md bg-success-3 border-l-4 border-success-8"
        ))
        .child(code_line(
            "813",
            "test('accept parses markdown replacements', () => {",
        ))
        .child(code_line("814", "  const schema = new Schema({"))
        .child(code_line("815", "    nodes: {"))
        .child(code_line("816", "      doc: { content: 'block+' },"))
        .child(code_line("817", "      paragraph: { content: 'inline*' },"))
        .child(code_line("818", "      text: { group: 'inline' },"))
}

fn hunk_tail() -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().fill_width().grow(1.0).gap(1.0))
        .class(classes!(
            "rounded-md bg-success-3 border-l-4 border-success-8"
        ))
        .child(code_line("821", "      proofSuggestion: {"))
        .child(code_line("822", "        attrs: {"))
        .child(code_line("823", "          id: { default: null },"))
        .child(code_line("824", "          kind: { default: 'replace' },"))
}

fn code_line(num: &str, src: &str) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .padding_x(8.0)
                .padding_y(2.0)
                .gap(10.0),
        )
        .child(label(num.to_owned()).class(classes!("text-xs text-gray-11")))
        .child(
            label(src.to_owned())
                .ellipsize()
                .layout(LayoutStyle::block().grow(1.0).min_size(0.0, 14.0))
                .class(classes!("text-xs text-gray-12")),
        )
}

fn comment(shell: Shell) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_column()
                .fill_width()
                .padding(10.0)
                .gap(8.0),
        )
        .class(classes!("rounded-md border border-gray-6 bg-gray-1"))
        .child(text_field(shell.note).placeholder("Comment on this hunk"))
        .child(
            container()
                .layout(
                    LayoutStyle::flex_row()
                        .fill_width()
                        .justify_content(JustifyContent::End)
                        .gap(8.0),
                )
                .child(button("Cancel").neutral())
                .child(button("Comment").primary()),
        )
}

fn spacer() -> NodeBuilder {
    container().layout(LayoutStyle::block().grow(1.0))
}
