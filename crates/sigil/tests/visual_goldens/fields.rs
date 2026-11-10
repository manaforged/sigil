use super::*;

#[test]
fn panel_default() {
    let img = Harness::render(|| panel().title("Settings"))
        .stub()
        .mode(InteractionMode::Default)
        .size([340, 92])
        .hit((1.0, 1.0))
        .image();
    Golden::new("panel_default").delta(DELTA_PCT).assert(img);
}

#[test]
fn progress_empty() {
    let img = Harness::render(|| {
        let v = signal(0.0_f32);
        progress(v, 100.0).height(24.0)
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([244, 32])
    .hit((1.0, 1.0))
    .image();
    Golden::new("progress_empty").delta(DELTA_PCT).assert(img);
}

#[test]
fn progress_half() {
    let img = Harness::render(|| {
        let v = signal(50.0_f32);
        progress(v, 100.0).height(24.0)
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([244, 32])
    .hit((1.0, 1.0))
    .image();
    Golden::new("progress_half").delta(DELTA_PCT).assert(img);
}

#[test]
fn progress_full() {
    let img = Harness::render(|| {
        let v = signal(100.0_f32);
        progress(v, 100.0).height(24.0)
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([244, 32])
    .hit((1.0, 1.0))
    .image();
    Golden::new("progress_full").delta(DELTA_PCT).assert(img);
}

fn dropdown_options() -> Vec<String> {
    vec!["Easy".into(), "Normal".into(), "Hard".into()]
}

#[test]
fn dropdown_open() {
    let img = Harness::render(|| {
        let sel = signal(0_usize);
        dropdown(sel, dropdown_options())
    })
    .stub()
    .mode(InteractionMode::Clicked)
    .size([212, 160])
    .hit((90.0, 22.0))
    .image();
    Golden::new("dropdown_open").delta(DELTA_PCT).assert(img);
}

#[test]
fn dropdown_default() {
    let img = Harness::render(|| {
        let sel = signal(0_usize);
        dropdown(sel, dropdown_options())
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([212, 52])
    .hit((90.0, 22.0))
    .image();
    Golden::new("dropdown_default").delta(DELTA_PCT).assert(img);
}

#[test]
fn dropdown_hover() {
    let img = Harness::render(|| {
        let sel = signal(0_usize);
        dropdown(sel, dropdown_options())
    })
    .stub()
    .mode(InteractionMode::Hover)
    .size([212, 52])
    .hit((90.0, 22.0))
    .image();
    Golden::new("dropdown_hover").delta(DELTA_PCT).assert(img);
}

#[test]
fn dropdown_pressed() {
    let img = Harness::render(|| {
        let sel = signal(0_usize);
        dropdown(sel, dropdown_options())
    })
    .stub()
    .mode(InteractionMode::Pressed)
    .size([212, 52])
    .hit((90.0, 22.0))
    .image();
    Golden::new("dropdown_pressed").delta(DELTA_PCT).assert(img);
}

#[test]
fn dropdown_focused() {
    let img = Harness::render(|| {
        let sel = signal(0_usize);
        dropdown(sel, dropdown_options())
    })
    .stub()
    .mode(InteractionMode::Focused)
    .size([212, 52])
    .hit((90.0, 22.0))
    .image();
    Golden::new("dropdown_focused").delta(DELTA_PCT).assert(img);
}

#[test]
fn dropdown_disabled() {
    let img = Harness::render(|| {
        let sel = signal(0_usize);
        let dis = signal(true);
        dropdown(sel, dropdown_options()).disabled(dis)
    })
    .stub()
    .mode(InteractionMode::Disabled)
    .size([212, 52])
    .hit((90.0, 22.0))
    .image();
    Golden::new("dropdown_disabled")
        .delta(DELTA_PCT)
        .assert(img);
}

#[test]
fn scroll_default() {
    let img = Harness::render(|| {
        let body = container()
            .background(Token::Accent)
            .radius(Radius::Sm)
            .layout(LayoutStyle::block().fixed_size(312.0, 400.0));
        scroll(body).extent(400.0).radius(Radius::Sm)
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([330, 252])
    .hit((1.0, 1.0))
    .image();
    Golden::new("scroll_default").delta(DELTA_PCT).assert(img);
}

#[test]
fn text_field_default() {
    let img = Harness::render(|| {
        let v = signal(String::from("Hello"));
        text_field(v)
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([192, 44])
    .hit((40.0, 22.0))
    .image();
    Golden::new("text_field_default")
        .delta(DELTA_PCT)
        .assert(img);
}

#[test]
fn text_field_hover() {
    let img = Harness::render(|| {
        let v = signal(String::from("Hello"));
        text_field(v)
    })
    .stub()
    .mode(InteractionMode::Hover)
    .size([192, 44])
    .hit((40.0, 22.0))
    .image();
    Golden::new("text_field_hover").delta(DELTA_PCT).assert(img);
}

#[test]
fn text_field_pressed() {
    let img = Harness::render(|| {
        let v = signal(String::from("Hello"));
        text_field(v)
    })
    .stub()
    .mode(InteractionMode::Pressed)
    .size([192, 44])
    .hit((40.0, 22.0))
    .image();
    Golden::new("text_field_pressed")
        .delta(DELTA_PCT)
        .assert(img);
}

#[test]
fn text_field_focused() {
    let img = Harness::render(|| {
        let v = signal(String::from("Hello"));
        text_field(v)
    })
    .stub()
    .mode(InteractionMode::Focused)
    .size([192, 44])
    .hit((40.0, 22.0))
    .image();
    Golden::new("text_field_focused")
        .delta(DELTA_PCT)
        .assert(img);
}

#[test]
fn text_field_disabled() {
    let img = Harness::render(|| {
        let v = signal(String::from("Hello"));
        let dis = signal(true);
        text_field(v).disabled(dis)
    })
    .stub()
    .mode(InteractionMode::Disabled)
    .size([192, 44])
    .hit((40.0, 22.0))
    .image();
    Golden::new("text_field_disabled")
        .delta(DELTA_PCT)
        .assert(img);
}

fn tab_labels() -> Vec<String> {
    vec!["First".into(), "Second".into(), "Third".into()]
}

#[test]
fn tabs_default() {
    let img = Harness::render(|| {
        let active = signal(0_usize);
        tabs().active(active).labels(tab_labels())
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([260, 56])
    .hit((40.0, 17.0))
    .image();
    Golden::new("tabs_default").delta(DELTA_PCT).assert(img);
}

#[test]
fn tabs_hover_tab2() {
    let img = Harness::render(|| {
        let active = signal(0_usize);
        tabs().active(active).labels(tab_labels())
    })
    .stub()
    .mode(InteractionMode::Hover)
    .size([260, 56])
    .hit((120.0, 17.0))
    .image();
    Golden::new("tabs_hover_tab2").delta(DELTA_PCT).assert(img);
}

#[test]
fn tabs_active_tab2() {
    let img = Harness::render(|| {
        let active = signal(1_usize);
        tabs().active(active).labels(tab_labels())
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([260, 56])
    .hit((120.0, 17.0))
    .image();
    Golden::new("tabs_active_tab2").delta(DELTA_PCT).assert(img);
}

#[test]
fn tabs_focused() {
    let img = Harness::render(|| {
        let active = signal(0_usize);
        tabs().active(active).labels(tab_labels())
    })
    .stub()
    .focus_first()
    .mode(InteractionMode::Focused)
    .size([260, 56])
    .hit((40.0, 17.0))
    .image();
    Golden::new("tabs_focused").delta(DELTA_PCT).assert(img);
}

#[test]
fn tabs_focused_differs_from_default() {
    Golden::new("tabs_focused").assert_differs(&Golden::new("tabs_default"));
}

#[test]
fn tabs_disabled() {
    let img = Harness::render(|| {
        let active = signal(0_usize);
        tabs().active(active).labels(tab_labels()).disabled(&[1])
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([260, 56])
    .hit((40.0, 17.0))
    .image();
    Golden::new("tabs_disabled").delta(DELTA_PCT).assert(img);
}
