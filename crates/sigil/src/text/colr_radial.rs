use tiny_skia::{Color, Point};

pub(super) type Stop = (f32, Color);

pub(super) struct Radial {
    pub c0: Point,
    pub r0: f32,
    pub c1: Point,
    pub r1: f32,
    pub stops: Vec<Stop>,
}

pub(super) fn cut_negative(radial: Radial) -> Option<Radial> {
    let (r0, r1) = (radial.r0, radial.r1);
    if r0 >= 0.0 && r1 >= 0.0 {
        return Some(radial);
    }
    if r0 <= 0.0 && r1 <= 0.0 {
        return None;
    }
    let t = r0 / (r0 - r1);
    let cut = Point::from_xy(
        lerp(radial.c0.x, radial.c1.x, t),
        lerp(radial.c0.y, radial.c1.y, t),
    );
    let edge = color_at(&radial.stops, t);
    if r0 < 0.0 {
        let mut stops = vec![(0.0, edge)];
        stops.extend(
            radial
                .stops
                .iter()
                .filter(|stop| stop.0 > t)
                .map(|stop| ((stop.0 - t) / (1.0 - t), stop.1)),
        );
        return Some(Radial {
            c0: cut,
            r0: 0.0,
            stops,
            ..radial
        });
    }
    let mut stops: Vec<Stop> = radial
        .stops
        .iter()
        .filter(|stop| stop.0 < t)
        .map(|stop| (stop.0 / t, stop.1))
        .collect();
    stops.push((1.0, edge));
    Some(Radial {
        c1: cut,
        r1: 0.0,
        stops,
        ..radial
    })
}

fn color_at(stops: &[Stop], t: f32) -> Color {
    let next = stops.iter().position(|stop| stop.0 >= t);
    match next {
        None => stops.last().map_or(Color::TRANSPARENT, |stop| stop.1),
        Some(0) => stops.first().map_or(Color::TRANSPARENT, |stop| stop.1),
        Some(index) => {
            let (Some(a), Some(b)) = (stops.get(index - 1), stops.get(index)) else {
                return Color::TRANSPARENT;
            };
            let span = b.0 - a.0;
            let f = if span > 0.0 { (t - a.0) / span } else { 0.0 };
            lerp_color(a.1, b.1, f)
        }
    }
}

fn lerp_color(a: Color, b: Color, f: f32) -> Color {
    Color::from_rgba(
        lerp(a.red(), b.red(), f),
        lerp(a.green(), b.green(), f),
        lerp(a.blue(), b.blue(), f),
        lerp(a.alpha(), b.alpha(), f),
    )
    .unwrap_or(a)
}

fn lerp(a: f32, b: f32, f: f32) -> f32 {
    (b - a).mul_add(f, a)
}
