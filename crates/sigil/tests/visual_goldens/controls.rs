use super::*;

#[test]
fn button_default() {
    let img = Harness::render(|| button("Save"))
        .stub()
        .mode(InteractionMode::Default)
        .size([160, 56])
        .hit((24.0, 16.0))
        .image();
    Golden::new("button_default").delta(DELTA_PCT).assert(img);
}

#[test]
fn button_solid_default() {
    let img = Harness::render(|| button("Save").primary())
        .stub()
        .mode(InteractionMode::Default)
        .size([160, 56])
        .hit((24.0, 16.0))
        .image();
    Golden::new("button_solid_default")
        .delta(DELTA_PCT)
        .assert(img);
}

#[test]
fn button_hover() {
    let img = Harness::render(|| button("Save"))
        .stub()
        .mode(InteractionMode::Hover)
        .size([160, 56])
        .hit((24.0, 16.0))
        .image();
    Golden::new("button_hover").delta(DELTA_PCT).assert(img);
}

#[test]
fn button_pressed() {
    let img = Harness::render(|| button("Save"))
        .stub()
        .mode(InteractionMode::Pressed)
        .size([160, 56])
        .hit((24.0, 16.0))
        .image();
    Golden::new("button_pressed").delta(DELTA_PCT).assert(img);
}

#[test]
fn button_focused() {
    let img = Harness::render(|| button("Save"))
        .stub()
        .mode(InteractionMode::Focused)
        .size([160, 56])
        .hit((24.0, 16.0))
        .image();
    Golden::new("button_focused").delta(DELTA_PCT).assert(img);
}

#[test]
fn button_disabled() {
    let img = Harness::render(|| button("Save").disabled(true))
        .stub()
        .mode(InteractionMode::Disabled)
        .size([160, 56])
        .hit((24.0, 16.0))
        .image();
    Golden::new("button_disabled").delta(DELTA_PCT).assert(img);
}

#[test]
fn checkbox_checked() {
    let img = Harness::render(|| {
        let checked = signal(true);
        checkbox(checked)
            .background(Token::Surface)
            .radius(Radius::Sm)
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([36, 36])
    .hit((16.0, 16.0))
    .image();
    Golden::new("checkbox_checked").delta(DELTA_PCT).assert(img);
}

#[test]
fn checkbox_default() {
    let img = Harness::render(|| {
        let checked = signal(false);
        checkbox(checked)
            .background(Token::Surface)
            .radius(Radius::Sm)
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([36, 36])
    .hit((16.0, 16.0))
    .image();
    Golden::new("checkbox_default").delta(DELTA_PCT).assert(img);
}

#[test]
fn checkbox_hover() {
    let img = Harness::render(|| {
        let checked = signal(false);
        checkbox(checked)
            .background(Token::Surface)
            .radius(Radius::Sm)
    })
    .stub()
    .mode(InteractionMode::Hover)
    .size([36, 36])
    .hit((16.0, 16.0))
    .image();
    Golden::new("checkbox_hover").delta(DELTA_PCT).assert(img);
}

#[test]
fn checkbox_pressed() {
    let img = Harness::render(|| {
        let checked = signal(false);
        checkbox(checked)
            .background(Token::Surface)
            .radius(Radius::Sm)
    })
    .stub()
    .mode(InteractionMode::Pressed)
    .size([36, 36])
    .hit((16.0, 16.0))
    .image();
    Golden::new("checkbox_pressed").delta(DELTA_PCT).assert(img);
}

#[test]
fn checkbox_focused() {
    let img = Harness::render(|| {
        let checked = signal(false);
        checkbox(checked)
            .background(Token::Surface)
            .radius(Radius::Sm)
    })
    .stub()
    .mode(InteractionMode::Focused)
    .size([36, 36])
    .hit((16.0, 16.0))
    .image();
    Golden::new("checkbox_focused").delta(DELTA_PCT).assert(img);
}

#[test]
fn checkbox_disabled() {
    let img = Harness::render(|| {
        let checked = signal(false);
        let dis = signal(true);
        checkbox(checked)
            .background(Token::Surface)
            .radius(Radius::Sm)
            .disabled(dis)
    })
    .stub()
    .mode(InteractionMode::Disabled)
    .size([36, 36])
    .hit((16.0, 16.0))
    .image();
    Golden::new("checkbox_disabled")
        .delta(DELTA_PCT)
        .assert(img);
}

#[test]
fn slider_default() {
    let img = Harness::render(|| {
        let v = signal(50.0_f32);
        slider(v, 0.0..=100.0)
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([192, 36])
    .hit((90.0, 14.0))
    .image();
    Golden::new("slider_default").delta(DELTA_PCT).assert(img);
}

#[test]
fn slider_hover() {
    let img = Harness::render(|| {
        let v = signal(50.0_f32);
        slider(v, 0.0..=100.0)
    })
    .stub()
    .mode(InteractionMode::Hover)
    .size([192, 36])
    .hit((90.0, 14.0))
    .image();
    Golden::new("slider_hover").delta(DELTA_PCT).assert(img);
}

#[test]
fn slider_pressed() {
    let img = Harness::render(|| {
        let v = signal(50.0_f32);
        slider(v, 0.0..=100.0)
    })
    .stub()
    .mode(InteractionMode::Pressed)
    .size([192, 36])
    .hit((90.0, 14.0))
    .image();
    Golden::new("slider_pressed").delta(DELTA_PCT).assert(img);
}

#[test]
fn slider_focused() {
    let img = Harness::render(|| {
        let v = signal(50.0_f32);
        slider(v, 0.0..=100.0)
    })
    .stub()
    .mode(InteractionMode::Focused)
    .size([192, 36])
    .hit((90.0, 14.0))
    .image();
    Golden::new("slider_focused").delta(DELTA_PCT).assert(img);
}

#[test]
fn slider_disabled() {
    let img = Harness::render(|| {
        let v = signal(50.0_f32);
        let dis = signal(true);
        slider(v, 0.0..=100.0).disabled(dis)
    })
    .stub()
    .mode(InteractionMode::Disabled)
    .size([192, 36])
    .hit((90.0, 14.0))
    .image();
    Golden::new("slider_disabled").delta(DELTA_PCT).assert(img);
}
