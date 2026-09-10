use sigil::prelude::*;

const fn ink() -> Rgba8 {
    Rgba8 {
        r: 40,
        g: 44,
        b: 52,
        a: 255,
    }
}

const fn snow() -> Rgba8 {
    Rgba8 {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    }
}

fn mark(w: u32, h: u32) -> PathNode {
    path(w, h).stroke(ink(), 1.6).fit_size(14.0)
}

fn fine(w: u32, h: u32) -> PathNode {
    path(w, h).stroke(ink(), 1.2).fit_size(14.0)
}

fn ring(node: PathNode, cx: f32, cy: f32, r: f32) -> PathNode {
    let k = r * 0.552_284_8;
    node.move_to(cx + r, cy)
        .cubic_to(cx + r, cy + k, cx + k, cy + r, cx, cy + r)
        .cubic_to(cx - k, cy + r, cx - r, cy + k, cx - r, cy)
        .cubic_to(cx - r, cy - k, cx - k, cy - r, cx, cy - r)
        .cubic_to(cx + k, cy - r, cx + r, cy - k, cx + r, cy)
        .close()
}

pub(crate) fn folder() -> PathNode {
    mark(16, 16)
        .move_to(2.0, 13.0)
        .line_to(2.0, 5.0)
        .line_to(6.0, 5.0)
        .line_to(8.0, 7.0)
        .line_to(14.0, 7.0)
        .line_to(14.0, 13.0)
        .close()
}

pub(crate) fn pin() -> PathNode {
    mark(16, 16)
        .move_to(8.0, 2.0)
        .line_to(10.0, 7.0)
        .line_to(14.0, 7.0)
        .line_to(10.5, 10.0)
        .line_to(12.0, 14.0)
        .line_to(8.0, 11.5)
        .line_to(4.0, 14.0)
        .line_to(5.5, 10.0)
        .line_to(2.0, 7.0)
        .line_to(6.0, 7.0)
        .close()
}

pub(crate) fn plus() -> PathNode {
    mark(16, 16)
        .move_to(8.0, 3.0)
        .line_to(8.0, 13.0)
        .move_to(3.0, 8.0)
        .line_to(13.0, 8.0)
}

pub(crate) fn compose() -> PathNode {
    mark(16, 16)
        .move_to(3.0, 13.0)
        .line_to(3.0, 10.5)
        .line_to(11.0, 2.5)
        .line_to(13.5, 5.0)
        .line_to(5.5, 13.0)
        .close()
        .move_to(9.6, 4.0)
        .line_to(12.0, 6.4)
}

pub(crate) fn clock() -> PathNode {
    ring(fine(16, 16), 8.0, 8.0, 6.2)
        .move_to(8.0, 4.5)
        .line_to(8.0, 8.0)
        .line_to(11.2, 10.0)
}

pub(crate) fn spark() -> PathNode {
    mark(16, 16)
        .move_to(8.0, 2.0)
        .line_to(9.5, 6.5)
        .line_to(14.0, 8.0)
        .line_to(9.5, 9.5)
        .line_to(8.0, 14.0)
        .line_to(6.5, 9.5)
        .line_to(2.0, 8.0)
        .line_to(6.5, 6.5)
        .close()
}

pub(crate) fn send() -> PathNode {
    path(16, 16)
        .stroke(snow(), 1.8)
        .move_to(8.0, 3.0)
        .line_to(8.0, 13.0)
        .move_to(4.0, 7.0)
        .line_to(8.0, 3.0)
        .line_to(12.0, 7.0)
        .fit_size(14.0)
}

pub(crate) fn chevron_left() -> PathNode {
    mark(16, 16)
        .move_to(10.0, 3.0)
        .line_to(5.0, 8.0)
        .line_to(10.0, 13.0)
}

pub(crate) fn chevron_down() -> PathNode {
    mark(16, 16)
        .move_to(4.0, 6.0)
        .line_to(8.0, 10.0)
        .line_to(12.0, 6.0)
}

pub(crate) fn play() -> PathNode {
    mark(16, 16)
        .move_to(5.0, 3.0)
        .line_to(13.0, 8.0)
        .line_to(5.0, 13.0)
        .close()
}

pub(crate) fn branch() -> PathNode {
    ring(mark(16, 16), 5.0, 4.0, 2.0)
        .move_to(5.0, 6.0)
        .line_to(5.0, 12.0)
        .move_to(5.0, 9.0)
        .cubic_to(5.0, 9.0, 11.0, 9.0, 11.0, 12.0)
}

pub(crate) fn calendar() -> PathNode {
    mark(16, 16)
        .move_to(3.0, 5.0)
        .line_to(13.0, 5.0)
        .line_to(13.0, 14.0)
        .line_to(3.0, 14.0)
        .close()
        .move_to(5.0, 3.0)
        .line_to(5.0, 6.0)
        .move_to(11.0, 3.0)
        .line_to(11.0, 6.0)
        .move_to(3.0, 8.0)
        .line_to(13.0, 8.0)
}

pub(crate) fn archive() -> PathNode {
    mark(16, 16)
        .move_to(2.0, 6.0)
        .line_to(14.0, 6.0)
        .line_to(13.0, 14.0)
        .line_to(3.0, 14.0)
        .close()
        .move_to(2.0, 3.0)
        .line_to(14.0, 3.0)
        .line_to(14.0, 6.0)
        .line_to(2.0, 6.0)
        .close()
}

pub(crate) fn filter() -> PathNode {
    mark(16, 16)
        .move_to(3.0, 4.0)
        .line_to(13.0, 4.0)
        .move_to(5.0, 8.0)
        .line_to(11.0, 8.0)
        .move_to(7.0, 12.0)
        .line_to(9.0, 12.0)
}

pub(crate) fn diffs() -> PathNode {
    mark(16, 16)
        .move_to(3.0, 4.0)
        .line_to(10.0, 4.0)
        .line_to(10.0, 13.0)
        .line_to(3.0, 13.0)
        .close()
        .move_to(6.0, 3.0)
        .line_to(13.0, 3.0)
        .line_to(13.0, 12.0)
        .line_to(10.0, 12.0)
}

pub(crate) fn avatar() -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().fixed_size(18.0, 18.0))
        .class(classes!("rounded-full bg-gray-8"))
}
