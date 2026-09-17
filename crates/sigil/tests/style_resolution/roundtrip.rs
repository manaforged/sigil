use sigil::style::{Style, StyleAtom};

#[allow(
    clippy::too_many_lines,
    reason = "exhaustive StyleAtom -> fluent-Style roundtrip — every atom must stay reachable from the struct API (DR-056 D3)"
)]
pub(super) fn style_for_atom(atom: StyleAtom) -> Style {
    match atom {
        StyleAtom::PaddingAll(px) => Style::new().padding(px.0),
        StyleAtom::PaddingX(px) => Style::new().px(px.0),
        StyleAtom::PaddingY(px) => Style::new().py(px.0),
        StyleAtom::PaddingTop(px) => Style::new().pt(px.0),
        StyleAtom::PaddingRight(px) => Style::new().pr(px.0),
        StyleAtom::PaddingBottom(px) => Style::new().pb(px.0),
        StyleAtom::PaddingLeft(px) => Style::new().pl(px.0),
        StyleAtom::MarginAll(px) => Style::new().margin(px.0),
        StyleAtom::MarginX(px) => Style::new().mx(px.0),
        StyleAtom::MarginY(px) => Style::new().my(px.0),
        StyleAtom::MarginTop(px) => Style::new().mt(px.0),
        StyleAtom::MarginRight(px) => Style::new().mr(px.0),
        StyleAtom::MarginBottom(px) => Style::new().mb(px.0),
        StyleAtom::MarginLeft(px) => Style::new().ml(px.0),
        StyleAtom::Gap(px) => Style::new().gap(px.0),
        StyleAtom::GapX(px) => Style::new().gap_x(px.0),
        StyleAtom::GapY(px) => Style::new().gap_y(px.0),
        StyleAtom::Width(dimension) => Style::new().width(dimension),
        StyleAtom::Height(dimension) => Style::new().height(dimension),
        StyleAtom::MinWidth(dimension) => Style::new().min_width(dimension),
        StyleAtom::MaxWidth(dimension) => Style::new().max_width(dimension),
        StyleAtom::MinHeight(dimension) => Style::new().min_height(dimension),
        StyleAtom::MaxHeight(dimension) => Style::new().max_height(dimension),
        StyleAtom::Display(display) => Style::new().display(display),
        StyleAtom::FlexDirection(direction) => Style::new().flex_direction(direction),
        StyleAtom::FlexWrap(wrap) => Style::new().flex_wrap(wrap),
        StyleAtom::AlignItems(align) => Style::new().align_items(align),
        StyleAtom::JustifyContent(justify) => Style::new().justify_content(justify),
        StyleAtom::FlexGrow(grow) => Style::new().flex_grow(grow),
        StyleAtom::FlexShrink(shrink) => Style::new().flex_shrink(shrink),
        StyleAtom::GridColumns(n) => Style::new().grid_columns(n),
        StyleAtom::GridRows(n) => Style::new().grid_rows(n),
        StyleAtom::BackgroundColor(color) => Style::new().fill(color),
        StyleAtom::BackgroundThemeRef(token) => Style::new().background_theme(token),
        StyleAtom::TextColor(color) => Style::new().text_color(color),
        StyleAtom::TextThemeRef(token) => Style::new().text_theme(token),
        StyleAtom::BorderRadius(px) => Style::new().radius(px.0),
        StyleAtom::BorderWidth(px) => Style::new().border_width(px.0),
        StyleAtom::BorderTop(px) => Style::new().border_top(px.0),
        StyleAtom::BorderRight(px) => Style::new().border_right(px.0),
        StyleAtom::BorderBottom(px) => Style::new().border_bottom(px.0),
        StyleAtom::BorderLeft(px) => Style::new().border_left(px.0),
        StyleAtom::BorderColor(color) => Style::new().border_color(color),
        StyleAtom::BorderThemeRef(token) => Style::new().border_theme(token),
        StyleAtom::Shadow(elevation) => Style::new().shadow(elevation),
        StyleAtom::Opacity(opacity) => Style::new().opacity_raw(opacity),
        StyleAtom::Transform(transform) => Style::new().transform(transform),
        StyleAtom::FontSize(px) => Style::new().font_size(px.0),
        StyleAtom::FontWeight(weight) => Style::new().font_weight(weight),
        StyleAtom::FontItalic(italic) => Style::new().font_italic(italic),
        StyleAtom::FontFamily(name) => Style::new().font_family(name),
        StyleAtom::LineHeight(px) => Style::new().line_height(px.0),
        StyleAtom::LetterSpacing(em) => Style::new().letter_spacing_em(f32::from(em) / 1000.0),
        StyleAtom::TabularNums(on) => Style::new().tabular_nums(on),
        StyleAtom::TextDecoration(decoration) => Style::new().text_decoration(decoration),
        StyleAtom::Transition(transition) => Style::new().transition(transition),
        StyleAtom::TransitionDuration(duration) => Style::new().duration(duration),
        StyleAtom::UnknownThemeRef(token) => Style::new().unknown_theme_ref(token),
        StyleAtom::OverflowX(kind) => Style::new().overflow_x(kind),
        StyleAtom::OverflowY(kind) => Style::new().overflow_y(kind),
        StyleAtom::BgToneStep(_)
        | StyleAtom::FgToneStep(_)
        | StyleAtom::FgOnSolid
        | StyleAtom::BorderToneStep(_) => {
            panic!("tone-parameterized atom {atom:?} has no fluent Style roundtrip")
        }
    }
}
