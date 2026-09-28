use sigil::prelude::*;

use super::spec::{Build, State, node};

pub(crate) const BUILDERS: &[(&str, Build)] = &[
    ("text", text_row),
    ("label", label_row),
    ("section_label", section_row),
    ("icon", icon_row),
    ("chip", chip_row),
    ("badge", badge_row),
    ("dot", dot_row),
    ("avatar", avatar_row),
    ("path", path_row),
    ("image_rgba", image_row),
    ("divider_horizontal", divider_h_row),
    ("divider_vertical", divider_v_row),
    ("hairline", hairline_row),
    ("notice_band", notice_row),
    ("empty_state", empty_row),
    ("hint_bar", hint_bar_row),
    ("markdown", markdown_row),
    ("markdown_image", markdown_image_row),
    ("mermaid", mermaid_row),
    ("html", html_row),
];

fn text_row(_state: State) -> NodeBuilder {
    node(text(
        "Text wraps to the width of its parent and flows onto the next line.",
    ))
}

fn label_row(_state: State) -> NodeBuilder {
    node(text("Labels and text share TextContent.").nowrap())
}

fn section_row(_state: State) -> NodeBuilder {
    node(section_label("Section"))
}

fn icon_row(_state: State) -> NodeBuilder {
    node(icon("▲").tone(Tone::Warn).label("Alert"))
}

fn chip_row(_state: State) -> NodeBuilder {
    node(chip("Stable", Tone::Success))
}

fn badge_row(_state: State) -> NodeBuilder {
    node(badge("4", Tone::Error))
}

fn dot_row(_state: State) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_row().gap(8.0))
        .child(dot(Tone::Success))
        .child(dot(Tone::Warn))
        .child(dot(Tone::Error))
}

fn avatar_row(_state: State) -> NodeBuilder {
    node(avatar("TG"))
}

fn path_row(_state: State) -> NodeBuilder {
    node(
        path(24, 24)
            .move_to(4.0, 20.0)
            .line_to(12.0, 4.0)
            .line_to(20.0, 20.0)
            .close()
            .fill(Rgba8 {
                r: 90,
                g: 140,
                b: 255,
                a: 255,
            })
            .fit(),
    )
}

fn image_row(_state: State) -> NodeBuilder {
    let pixels: Vec<u8> = (0..32_u32 * 32)
        .flat_map(|i| {
            let shade = u8::try_from((i % 32) * 8).unwrap_or(u8::MAX);
            [shade, 120, 255 - shade, 255]
        })
        .collect();
    image_rgba(32, 32, pixels).map_or_else(|| node(text("image_rgba rejected").nowrap()), node)
}

fn divider_h_row(_state: State) -> NodeBuilder {
    node(divider_horizontal())
}

fn divider_v_row(_state: State) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_row().height(40.0))
        .child(divider_vertical())
}

fn hairline_row(_state: State) -> NodeBuilder {
    node(hairline())
}

fn notice_row(_state: State) -> NodeBuilder {
    node(notice_band("Deployment finished", Tone::Success))
}

fn empty_row(_state: State) -> NodeBuilder {
    node(empty_state("No results", "Try a different search term."))
}

fn hint_bar_row(_state: State) -> NodeBuilder {
    node(hint_bar("↑↓ navigate", "Enter select"))
}

fn markdown_row(_state: State) -> NodeBuilder {
    node(markdown(signal(String::from(
        "# Heading\n\nBody with **bold** and `code`.\n\n- one\n- two",
    ))))
}

const QUADRANTS: &str = "iVBORw0KGgoAAAANSUhEUgAAAGAAAAAwCAYAAADuFn/PAAAAeklEQVR42u3SQQ0AQAgDQYQh7MTihVPBo8k8VkAzrene5PpNdAUAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAwAFA+oD0AwEAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAcNAHChr0hrUljjwAAAAASUVORK5CYII=";

fn markdown_image_row(_state: State) -> NodeBuilder {
    node(markdown(signal(format!(
        "Image:\n\n![quadrants](data:image/png;base64,{QUADRANTS})"
    ))))
}

fn mermaid_row(_state: State) -> NodeBuilder {
    node(mermaid("graph TD\n  Edit --> Parse\n  Parse --> Paint"))
}

fn html_row(_state: State) -> NodeBuilder {
    node(html(signal(String::from(
        "<h2>Release notes</h2><p>Sigil renders <b>HTML</b> with <i>inline</i> styles.</p>",
    ))))
}
