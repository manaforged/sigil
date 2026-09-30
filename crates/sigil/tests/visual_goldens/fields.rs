use super::*;

#[test]
fn panel_default() {
    let img = render_widget_to_png_stub(
        || panel().title("Settings"),
        InteractionMode::Default,
        [340, 92],
        (1.0, 1.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("panel_default"), DELTA_PCT);
}

#[test]
fn progress_empty() {
    let img = render_widget_to_png_stub(
        || {
            let v = signal(0.0_f32);
            progress(v, 100.0).height(24.0)
        },
        InteractionMode::Default,
        [244, 32],
        (1.0, 1.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("progress_empty"), DELTA_PCT);
}

#[test]
fn progress_half() {
    let img = render_widget_to_png_stub(
        || {
            let v = signal(50.0_f32);
            progress(v, 100.0).height(24.0)
        },
        InteractionMode::Default,
        [244, 32],
        (1.0, 1.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("progress_half"), DELTA_PCT);
}

#[test]
fn progress_full() {
    let img = render_widget_to_png_stub(
        || {
            let v = signal(100.0_f32);
            progress(v, 100.0).height(24.0)
        },
        InteractionMode::Default,
        [244, 32],
        (1.0, 1.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("progress_full"), DELTA_PCT);
}

fn dropdown_options() -> Vec<String> {
    vec!["Easy".into(), "Normal".into(), "Hard".into()]
}

#[test]
fn dropdown_open() {
    let img = render_widget_to_png_stub(
        || {
            let sel = signal(0_usize);
            dropdown(sel, dropdown_options())
        },
        InteractionMode::Clicked,
        [212, 160],
        (90.0, 22.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("dropdown_open"), DELTA_PCT);
}

#[test]
fn dropdown_default() {
    let img = render_widget_to_png_stub(
        || {
            let sel = signal(0_usize);
            dropdown(sel, dropdown_options())
        },
        InteractionMode::Default,
        [212, 52],
        (90.0, 22.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("dropdown_default"), DELTA_PCT);
}

#[test]
fn dropdown_hover() {
    let img = render_widget_to_png_stub(
        || {
            let sel = signal(0_usize);
            dropdown(sel, dropdown_options())
        },
        InteractionMode::Hover,
        [212, 52],
        (90.0, 22.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("dropdown_hover"), DELTA_PCT);
}

#[test]
fn dropdown_pressed() {
    let img = render_widget_to_png_stub(
        || {
            let sel = signal(0_usize);
            dropdown(sel, dropdown_options())
        },
        InteractionMode::Pressed,
        [212, 52],
        (90.0, 22.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("dropdown_pressed"), DELTA_PCT);
}

#[test]
fn dropdown_focused() {
    let img = render_widget_to_png_stub(
        || {
            let sel = signal(0_usize);
            dropdown(sel, dropdown_options())
        },
        InteractionMode::Focused,
        [212, 52],
        (90.0, 22.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("dropdown_focused"), DELTA_PCT);
}

#[test]
fn dropdown_disabled() {
    let img = render_widget_to_png_stub(
        || {
            let sel = signal(0_usize);
            let dis = signal(true);
            dropdown(sel, dropdown_options()).disabled(dis)
        },
        InteractionMode::Disabled,
        [212, 52],
        (90.0, 22.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("dropdown_disabled"), DELTA_PCT);
}

#[test]
fn scroll_default() {
    let img = render_widget_to_png_stub(
        || {
            let body = container()
                .class("bg-accent rounded-sm")
                .layout(LayoutStyle::block().fixed_size(312.0, 400.0));
            scroll(body).extent(400.0).class("rounded-sm")
        },
        InteractionMode::Default,
        [330, 252],
        (1.0, 1.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("scroll_default"), DELTA_PCT);
}

#[test]
fn text_field_default() {
    let img = render_widget_to_png_stub(
        || {
            let v = signal(String::from("Hello"));
            text_field(v)
        },
        InteractionMode::Default,
        [192, 44],
        (40.0, 22.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("text_field_default"), DELTA_PCT);
}

#[test]
fn text_field_hover() {
    let img = render_widget_to_png_stub(
        || {
            let v = signal(String::from("Hello"));
            text_field(v)
        },
        InteractionMode::Hover,
        [192, 44],
        (40.0, 22.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("text_field_hover"), DELTA_PCT);
}

#[test]
fn text_field_pressed() {
    let img = render_widget_to_png_stub(
        || {
            let v = signal(String::from("Hello"));
            text_field(v)
        },
        InteractionMode::Pressed,
        [192, 44],
        (40.0, 22.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("text_field_pressed"), DELTA_PCT);
}

#[test]
fn text_field_focused() {
    let img = render_widget_to_png_stub(
        || {
            let v = signal(String::from("Hello"));
            text_field(v)
        },
        InteractionMode::Focused,
        [192, 44],
        (40.0, 22.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("text_field_focused"), DELTA_PCT);
}

#[test]
fn text_field_disabled() {
    let img = render_widget_to_png_stub(
        || {
            let v = signal(String::from("Hello"));
            let dis = signal(true);
            text_field(v).disabled(dis)
        },
        InteractionMode::Disabled,
        [192, 44],
        (40.0, 22.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("text_field_disabled"), DELTA_PCT);
}

fn tab_labels() -> Vec<String> {
    vec!["First".into(), "Second".into(), "Third".into()]
}

#[test]
fn tabs_default() {
    let img = render_widget_to_png_stub(
        || {
            let active = signal(0_usize);
            tabs().active(active).labels(tab_labels())
        },
        InteractionMode::Default,
        [260, 56],
        (40.0, 17.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("tabs_default"), DELTA_PCT);
}

#[test]
fn tabs_hover_tab2() {
    let img = render_widget_to_png_stub(
        || {
            let active = signal(0_usize);
            tabs().active(active).labels(tab_labels())
        },
        InteractionMode::Hover,
        [260, 56],
        (120.0, 17.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("tabs_hover_tab2"), DELTA_PCT);
}

#[test]
fn tabs_active_tab2() {
    let img = render_widget_to_png_stub(
        || {
            let active = signal(1_usize);
            tabs().active(active).labels(tab_labels())
        },
        InteractionMode::Default,
        [260, 56],
        (120.0, 17.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("tabs_active_tab2"), DELTA_PCT);
}

#[test]
fn tabs_focused() {
    let img = render_widget_to_png_focus_descendant(
        || {
            let active = signal(0_usize);
            tabs().active(active).labels(tab_labels())
        },
        InteractionMode::Focused,
        [260, 56],
        (40.0, 17.0),
        sigil_kittest::stub_text_measure(),
    );
    assert_rendered_matches_golden(img, &golden_path!("tabs_focused"), DELTA_PCT);
}

#[test]
fn tabs_focused_differs_from_default() {
    assert_goldens_differ!("tabs_focused", "tabs_default");
}

#[test]
fn tabs_disabled() {
    let img = render_widget_to_png_stub(
        || {
            let active = signal(0_usize);
            tabs().active(active).labels(tab_labels()).disabled(&[1])
        },
        InteractionMode::Default,
        [260, 56],
        (40.0, 17.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("tabs_disabled"), DELTA_PCT);
}
