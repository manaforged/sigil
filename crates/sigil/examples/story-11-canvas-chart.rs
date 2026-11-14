use sigil::prelude::*;
use sigil::scene::{Clip, DrawRect, SceneBuilder, Transform};
use sigil::style::TextStyle;

fn main() -> sigil::AppResult<()> {
    sigil::run("Canvas Chart", app)
}

/// Interactive Layer 2 chart screen exercising `canvas`, `SceneBuilder`, and `PathBuilder`.
#[must_use]
pub fn app() -> impl IntoNode {
    let samples = signal(vec![18.0_f32, 42.0, 29.0, 56.0, 38.0]);
    let version = signal(0_u64);

    column()
        .padding(Space::Lg)
        .gap(Space::Md)
        .background(Token::Surface)
        .child(text("Canvas Chart").text_role(TextRole::Title))
        .child(
            canvas(move |scene, rect, theme| {
                let values = samples.peek();
                paint_chart(scene, rect, theme, &values);
            })
            .redraw_on(version)
            .width(320.0)
            .height(140.0),
        )
        .child(
            row()
                .align_items(AlignItems::Center)
                .gap(Space::Md)
                .child(button("Add sample").primary().activate(move || {
                    samples.update(|list| {
                        let step = f32::from(u8::try_from(list.len() % 5).unwrap_or(0));
                        list.push(step.mul_add(11.0, 24.0));
                    });
                    version.update(|n| *n += 1);
                }))
                .child(text(move || format!("Samples: {}", samples.get().len()))),
        )
}

fn paint_chart(scene: &mut SceneBuilder, rect: Rect, theme: &Theme, values: &[f32]) {
    let surface = theme.color(Token::SurfaceSubtle);
    let accent = theme.color(Token::Accent);
    let border = theme.color(Token::BorderSubtle);
    let ink = theme.color(Token::OnSurface);

    scene.clip(Clip::rounded(rect, 8.0), |scene| {
        scene.draw_rect(DrawRect::new(rect, surface).radius(8.0));
        let inset = 10.0;
        let inner_w = (rect.width - inset * 2.0).max(1.0);
        let inner_h = (rect.height - inset * 2.0).max(1.0);
        let translate: Transform = [
            [1.0, 0.0, rect.origin.x + inset],
            [0.0, 1.0, rect.origin.y + inset],
        ];

        scene.transform(translate, |scene| {
            let header_h = draw_header(scene, inner_w, ink, values);
            let plot_y = header_h + 6.0;
            let plot_h = (inner_h - plot_y).max(1.0);
            let plot_rect = Rect::new(Point::new(0.0, plot_y), inner_w, plot_h);
            scene.stroke_line(
                Point::new(0.0, plot_y + plot_h),
                Point::new(inner_w, plot_y + plot_h),
                border,
                1.0,
            );
            draw_bars(scene, plot_rect, accent, values);
            draw_sparkline(scene, plot_rect, theme, values);
        });
    });
}

fn draw_header(scene: &mut SceneBuilder, width: f32, ink: Color, values: &[f32]) -> f32 {
    let peak = values.iter().copied().fold(0.0_f32, f32::max);
    let label = format!("Peak {peak:.0}");
    let style = TextStyle::default().font_size(12.0);
    let size = scene.measure_text(&label, &style);
    let x = (width - size.width).max(0.0);
    scene.draw_text(Point::new(x, 0.0), &label, &style, ink);
    size.height
}

#[allow(
    clippy::cast_precision_loss,
    reason = "sample counts fit cleanly within f32 mantissa"
)]
fn draw_bars(scene: &mut SceneBuilder, plot: Rect, accent: Color, values: &[f32]) {
    if values.is_empty() {
        return;
    }
    let peak = values.iter().copied().fold(1.0_f32, f32::max);
    let slot_w = plot.width / values.len() as f32;
    let bar_w = (slot_w - 6.0).max(2.0);
    let bar_color = Color { a: 110, ..accent };
    for (index, &value) in values.iter().enumerate() {
        let ratio = (value / peak).clamp(0.05, 1.0);
        let height = (plot.height * ratio).max(2.0);
        let x = (slot_w - bar_w).mul_add(0.5, (index as f32).mul_add(slot_w, plot.origin.x));
        let y = plot.origin.y + plot.height - height;
        let bar = Rect::new(Point::new(x, y), bar_w, height);
        scene.draw_rect(DrawRect::new(bar, bar_color).radius(4.0));
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "sample counts fit cleanly within f32 mantissa"
)]
fn draw_sparkline(scene: &mut SceneBuilder, plot: Rect, theme: &Theme, values: &[f32]) {
    if values.len() < 2 {
        return;
    }
    let peak = values.iter().copied().fold(1.0_f32, f32::max);
    let span = (values.len() - 1) as f32;
    let mut line = path(100, 100).stroke(Token::Accent, 2.0);
    for (index, &value) in values.iter().enumerate() {
        let x = (index as f32 / span) * 100.0;
        let y = (value / peak).clamp(0.0, 1.0).mul_add(-90.0, 100.0);
        line = if index == 0 {
            line.move_to(x, y)
        } else {
            line.line_to(x, y)
        };
    }
    if let Some(draw) = line.build_path(plot, theme) {
        scene.draw_path(draw);
    }
}
