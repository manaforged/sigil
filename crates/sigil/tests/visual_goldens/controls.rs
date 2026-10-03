use super::*;

#[test]
fn button_default() {
    let img = render_widget_to_png_stub(
        || button("Save"),
        InteractionMode::Default,
        [160, 56],
        (24.0, 16.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("button_default"), DELTA_PCT);
}

#[test]
fn button_solid_default() {
    let img = render_widget_to_png_stub(
        || button("Save").primary(),
        InteractionMode::Default,
        [160, 56],
        (24.0, 16.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("button_solid_default"), DELTA_PCT);
}

#[test]
fn button_hover() {
    let img = render_widget_to_png_stub(
        || button("Save"),
        InteractionMode::Hover,
        [160, 56],
        (24.0, 16.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("button_hover"), DELTA_PCT);
}

#[test]
fn button_pressed() {
    let img = render_widget_to_png_stub(
        || button("Save"),
        InteractionMode::Pressed,
        [160, 56],
        (24.0, 16.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("button_pressed"), DELTA_PCT);
}

#[test]
fn button_focused() {
    let img = render_widget_to_png_stub(
        || button("Save"),
        InteractionMode::Focused,
        [160, 56],
        (24.0, 16.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("button_focused"), DELTA_PCT);
}

#[test]
fn button_disabled() {
    let img = render_widget_to_png_stub(
        || button("Save").disabled(true),
        InteractionMode::Disabled,
        [160, 56],
        (24.0, 16.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("button_disabled"), DELTA_PCT);
}

#[test]
fn checkbox_checked() {
    let img = render_widget_to_png_stub(
        || {
            let checked = signal(true);
            checkbox(checked).bg(Token::Surface).rounded(Radius::Sm)
        },
        InteractionMode::Default,
        [36, 36],
        (16.0, 16.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("checkbox_checked"), DELTA_PCT);
}

#[test]
fn checkbox_default() {
    let img = render_widget_to_png_stub(
        || {
            let checked = signal(false);
            checkbox(checked).bg(Token::Surface).rounded(Radius::Sm)
        },
        InteractionMode::Default,
        [36, 36],
        (16.0, 16.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("checkbox_default"), DELTA_PCT);
}

#[test]
fn checkbox_hover() {
    let img = render_widget_to_png_stub(
        || {
            let checked = signal(false);
            checkbox(checked).bg(Token::Surface).rounded(Radius::Sm)
        },
        InteractionMode::Hover,
        [36, 36],
        (16.0, 16.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("checkbox_hover"), DELTA_PCT);
}

#[test]
fn checkbox_pressed() {
    let img = render_widget_to_png_stub(
        || {
            let checked = signal(false);
            checkbox(checked).bg(Token::Surface).rounded(Radius::Sm)
        },
        InteractionMode::Pressed,
        [36, 36],
        (16.0, 16.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("checkbox_pressed"), DELTA_PCT);
}

#[test]
fn checkbox_focused() {
    let img = render_widget_to_png_stub(
        || {
            let checked = signal(false);
            checkbox(checked).bg(Token::Surface).rounded(Radius::Sm)
        },
        InteractionMode::Focused,
        [36, 36],
        (16.0, 16.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("checkbox_focused"), DELTA_PCT);
}

#[test]
fn checkbox_disabled() {
    let img = render_widget_to_png_stub(
        || {
            let checked = signal(false);
            let dis = signal(true);
            checkbox(checked)
                .bg(Token::Surface)
                .rounded(Radius::Sm)
                .disabled(dis)
        },
        InteractionMode::Disabled,
        [36, 36],
        (16.0, 16.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("checkbox_disabled"), DELTA_PCT);
}

#[test]
fn slider_default() {
    let img = render_widget_to_png_stub(
        || {
            let v = signal(50.0_f32);
            slider(v, 0.0..=100.0)
        },
        InteractionMode::Default,
        [192, 36],
        (90.0, 14.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("slider_default"), DELTA_PCT);
}

#[test]
fn slider_hover() {
    let img = render_widget_to_png_stub(
        || {
            let v = signal(50.0_f32);
            slider(v, 0.0..=100.0)
        },
        InteractionMode::Hover,
        [192, 36],
        (90.0, 14.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("slider_hover"), DELTA_PCT);
}

#[test]
fn slider_pressed() {
    let img = render_widget_to_png_stub(
        || {
            let v = signal(50.0_f32);
            slider(v, 0.0..=100.0)
        },
        InteractionMode::Pressed,
        [192, 36],
        (90.0, 14.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("slider_pressed"), DELTA_PCT);
}

#[test]
fn slider_focused() {
    let img = render_widget_to_png_stub(
        || {
            let v = signal(50.0_f32);
            slider(v, 0.0..=100.0)
        },
        InteractionMode::Focused,
        [192, 36],
        (90.0, 14.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("slider_focused"), DELTA_PCT);
}

#[test]
fn slider_disabled() {
    let img = render_widget_to_png_stub(
        || {
            let v = signal(50.0_f32);
            let dis = signal(true);
            slider(v, 0.0..=100.0).disabled(dis)
        },
        InteractionMode::Disabled,
        [192, 36],
        (90.0, 14.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("slider_disabled"), DELTA_PCT);
}
