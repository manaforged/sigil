#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

#[path = "../examples/story-14-theming.rs"]
#[allow(dead_code, unreachable_pub)]
mod story_14;

use sigil::Host;
use sigil::app::{FrameHooks, HostMode, system_backdrop};
use sigil::events::{Event, WindowEvent};
use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil::scene::{LayerEffect, SceneCommand, Transform};
use sigil::style::Backdrop;
use sigil_kittest::Harness;

const VIEW: (f32, f32) = (480.0, 320.0);

fn hosts(app: fn() -> Harness) -> impl Iterator<Item = Harness> {
    [false, true].into_iter().map(move |pixels| {
        if pixels {
            app().pixels(480, 320)
        } else {
            app()
        }
    })
}

fn first_transform(scene: &sigil::scene::CommandScene) -> Transform {
    scene
        .commands()
        .iter()
        .find_map(|cmd| match cmd {
            SceneCommand::PushTransform(transform) => Some(*transform),
            SceneCommand::PushLayer(scope) => match *scope.effect() {
                LayerEffect::Transform(transform) => Some(transform),
                _ => None,
            },
            _ => None,
        })
        .expect("scene must emit a 2D transform")
}

fn emit_transform(build: impl Fn() -> NodeBuilder + 'static) -> Transform {
    let mut ui = Harness::new(move || {
        container()
            .width(200.0)
            .height(200.0)
            .child(build().width(100.0).height(100.0).background(Color::WHITE))
    })
    .pixels(200, 200);
    let scene = ui.host().emit((200.0, 200.0));
    first_transform(&scene)
}

fn assert_near(actual: f32, expected: f32, label: &str) {
    assert!(
        (actual - expected).abs() < 1e-4,
        "{label}: expected {expected}, got {actual}",
    );
}

#[test]
fn custom_token_resolves_and_paints_on_nodes() {
    Owner::scope("story-14-custom-token", || {
        const BRAND_SURFACE: Token = Token::Custom("brand-surface");
        let brand_color = Color::rgb(12, 110, 140);
        let theme = Theme::light().token(BRAND_SURFACE, brand_color);
        assert_eq!(theme.color(BRAND_SURFACE), brand_color);

        let mut host = Host::new(HostMode::Pixel);
        host.mount(
            container()
                .width(120.0)
                .height(48.0)
                .theme(theme)
                .background(BRAND_SURFACE),
        );
        let scene = host.scene(VIEW).expect("pixel scene");
        assert!(
            scene
                .commands()
                .iter()
                .any(|cmd| matches!(cmd, SceneCommand::Rect(rect) if rect.color() == brand_color)),
            "Token::Custom must paint its configured color",
        );
    });
}

#[test]
fn theme_metrics_and_radius_update_draw_rect_corner_radii() {
    Owner::scope("story-14-metrics-and-radius", || {
        let theme = Theme::light()
            .control_height(ControlSize::Default, 44.0)
            .radius(Radius::Md, 14.0);
        let metrics = theme.metrics();
        assert_near(
            metrics.control_height(ControlSize::Default),
            44.0,
            "control_height",
        );
        assert_near(metrics.radius(Radius::Md), 14.0, "radius");

        let mut host = Host::new(HostMode::Pixel);
        host.mount(
            container()
                .width(120.0)
                .height(44.0)
                .theme(theme)
                .background(Color::rgb(20, 40, 80))
                .radius(Radius::Md),
        );
        let scene = host.scene(VIEW).expect("pixel scene");
        assert!(
            scene.commands().iter().any(
                |cmd| matches!(cmd, SceneCommand::Rect(rect) if rect.corner_radii() == [14.0; 4])
            ),
            "DrawRect corner radii must reflect Theme::radius(Radius::Md, 14.0)",
        );
    });
}

fn rect_styles(host: &mut Host) -> Vec<(Color, [f32; 4])> {
    let _ = host.frame(VIEW, 1.0 / 60.0, FrameHooks::default());
    host.emit(VIEW)
        .commands()
        .iter()
        .filter_map(|cmd| match cmd {
            SceneCommand::Rect(rect) => Some((rect.color(), rect.corner_radii())),
            _ => None,
        })
        .collect()
}

#[test]
fn reactive_subtree_theme_updates_live_without_affecting_siblings() {
    Owner::scope("story-14-reactive-subtree-theme", || {
        let sibling_color = Color::rgb(10, 20, 30);
        let first_color = Color::rgb(180, 30, 50);
        let second_color = Color::rgb(30, 150, 70);
        let root_theme = Theme::light()
            .token(Token::Accent, sibling_color)
            .radius(Radius::Md, 6.0);
        let themed = |color, radius| {
            Theme::light()
                .token(Token::Accent, color)
                .radius(Radius::Md, radius)
        };
        let subtree_theme = signal(themed(first_color, 12.0));
        let box_node = || {
            container()
                .width(80.0)
                .height(32.0)
                .background(Token::Accent)
                .radius(Radius::Md)
        };

        let mut host = Host::new(HostMode::Pixel);
        host.mount(
            column()
                .theme(root_theme)
                .child(box_node())
                .child(container().theme(subtree_theme).child(box_node())),
        );

        let before = rect_styles(&mut host);
        assert!(before.contains(&(sibling_color, [6.0; 4])));
        assert!(before.contains(&(first_color, [12.0; 4])));
        assert!(!before.contains(&(second_color, [20.0; 4])));

        subtree_theme.set(themed(second_color, 20.0));
        let after = rect_styles(&mut host);
        assert!(after.contains(&(sibling_color, [6.0; 4])));
        assert!(!after.contains(&(first_color, [12.0; 4])));
        assert!(after.contains(&(second_color, [20.0; 4])));
    });
}

#[test]
fn for_backdrop_and_window_backdrop_changed_work_end_to_end() {
    assert_eq!(Theme::for_backdrop(Backdrop::Light), Theme::light());
    assert_eq!(Theme::for_backdrop(Backdrop::Dark), Theme::dark());

    Owner::scope("story-14-backdrop-event", || {
        let sys = system_backdrop();
        let mut host = Host::new(HostMode::Pixel);
        host.mount(
            column()
                .theme(move || sys.get().map_or_else(Theme::light, Theme::for_backdrop))
                .background(Token::Surface)
                .child(text(move || match sys.get() {
                    Some(Backdrop::Dark) => "System: Dark",
                    Some(Backdrop::Light) => "System: Light",
                    None => "System: None",
                })),
        );
        let _ = host.frame(VIEW, 1.0 / 60.0, FrameHooks::default());

        host.route(Event::Window(WindowEvent::BackdropChanged(Backdrop::Light)));
        let _ = host.frame(VIEW, 1.0 / 60.0, FrameHooks::default());
        assert_eq!(sys.peek(), Some(Backdrop::Light));
        assert!(host.emit(VIEW).find_text("System: Light").is_some());

        host.route(Event::Window(WindowEvent::BackdropChanged(Backdrop::Dark)));
        let _ = host.frame(VIEW, 1.0 / 60.0, FrameHooks::default());
        assert_eq!(sys.peek(), Some(Backdrop::Dark));
        assert!(host.emit(VIEW).find_text("System: Dark").is_some());
    });
}

#[test]
fn paint_2d_transforms_emit_push_transform() {
    let tx = emit_transform(|| container().translate_x(12.0));
    assert_near(tx[0][2], 12.0, "translate_x");
    assert_near(tx[1][2], 0.0, "translate_x y");

    let ty = emit_transform(|| container().translate_y(8.0));
    assert_near(ty[0][2], 0.0, "translate_y x");
    assert_near(ty[1][2], 8.0, "translate_y");

    let tr = emit_transform(|| container().translate(10.0, -4.0));
    assert_near(tr[0][2], 10.0, "translate x");
    assert_near(tr[1][2], -4.0, "translate y");

    let sc = emit_transform(|| container().scale(1.5));
    assert_near(sc[0][0], 1.5, "scale x");
    assert_near(sc[1][1], 1.5, "scale y");

    let sx = emit_transform(|| container().scale_x(2.0));
    assert_near(sx[0][0], 2.0, "scale_x");
    assert_near(sx[1][1], 1.0, "scale_x y");

    let sy = emit_transform(|| container().scale_y(0.5));
    assert_near(sy[0][0], 1.0, "scale_y x");
    assert_near(sy[1][1], 0.5, "scale_y");

    let rot = emit_transform(|| container().rotate(90.0));
    assert_near(rot[0][0], 0.0, "rotate 90 cos");
    assert_near(rot[0][1], -1.0, "rotate 90 -sin");
    assert_near(rot[1][0], 1.0, "rotate 90 sin");
    assert_near(rot[1][1], 0.0, "rotate 90 cos");
}

#[test]
fn paint_2d_transforms_compose_per_axis_across_hover() {
    let mut ui = Harness::new(|| {
        container().width(200.0).height(200.0).child(
            container()
                .width(100.0)
                .height(100.0)
                .background(Color::WHITE)
                .translate_x(10.0)
                .scale_y(2.0)
                .hover(|s| s.translate_y(6.0).scale_x(1.5)),
        )
    })
    .pixels(200, 200);

    let rest = first_transform(&ui.host().emit((200.0, 200.0)));
    assert_near(rest[0][0], 1.0, "rest scale_x");
    assert_near(rest[1][1], 2.0, "rest scale_y");
    assert_near(rest[0][2], 10.0, "rest tx");
    assert_near(rest[1][2], -50.0, "rest ty");

    ui.send([Event::pointer_move(Point::new(50.0, 50.0))]);
    let hovered = first_transform(&ui.host().emit((200.0, 200.0)));
    assert_near(hovered[0][0], 1.5, "hovered scale_x");
    assert_near(hovered[1][1], 2.0, "hovered scale_y preserved");
    assert_near(hovered[0][2], -15.0, "hovered tx preserved");
    assert_near(hovered[1][2], -44.0, "hovered ty");
}

#[test]
fn theming_story_app_works_on_both_hosts() {
    for mut ui in hosts(|| Harness::new(story_14::app)) {
        ui.expect_text("Theming & Subtrees")
            .click("Dark")
            .expect_text("Mode: Dark")
            .click("Light")
            .expect_text("Mode: Light")
            .click("System")
            .expect_text("Mode: System")
            .expect_text("BRAND")
            .click("Accent action")
            .expect_text("Action clicks: 1");
    }
}
