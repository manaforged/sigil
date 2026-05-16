//! SVG parsing via `usvg` — returns flattened path segments.
//!
//! Stroke handling: Lucide-style SVGs declare `fill="none"` plus a positive
//! `stroke-width`. The on-screen content is the stroked outline of the path,
//! not its interior winding. The icon baker's rasterizer is a winding-fill
//! engine, so this module expands every `Some(stroke)` path into its filled
//! outline via `tiny_skia_path::PathStroker` before handing it to the
//! quadratic-conversion stage. Fill-only paths (e.g. Heroicons-solid)
//! pass through unchanged.

use usvg::tiny_skia_path::PathSegment;

use crate::BakeError;

/// A parsed, transform-flattened contour from the SVG tree.
pub(crate) struct SvgPath {
    pub(crate) segments: Vec<PathSegment>,
}

/// `ViewBox` extracted from the SVG root element.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ViewBox {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

/// Parse `svg_source` and return flattened paths plus the root viewBox.
///
/// `usvg` applies all transforms during tree construction, so leaf path
/// nodes carry pre-flattened coordinates in the viewBox coordinate space.
pub(crate) fn parse(svg_source: &str) -> Result<(Vec<SvgPath>, ViewBox), BakeError> {
    let opts = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg_source, &opts)
        .map_err(|e| BakeError::SvgParseError(e.to_string()))?;

    let vb = tree.size();
    let view_box = ViewBox {
        x: 0.0,
        y: 0.0,
        width: vb.width(),
        height: vb.height(),
    };

    if view_box.width <= 0.0 || view_box.height <= 0.0 {
        return Err(BakeError::ViewBoxMissing);
    }

    let mut paths = Vec::new();
    collect_paths(tree.root(), &mut paths);
    Ok((paths, view_box))
}

fn collect_paths(parent: &usvg::Group, out: &mut Vec<SvgPath>) {
    for child in parent.children() {
        match child {
            usvg::Node::Path(path) => {
                push_fill_and_stroke(path, out);
            }
            usvg::Node::Group(g) => collect_paths(g, out),
            _ => {}
        }
    }
}

/// Emit one `SvgPath` per renderable contribution from `path`.
///
/// usvg paths can carry both a fill and a stroke. The rasterizer is a
/// winding-fill engine, so the stroked outline is materialised as its
/// own filled contour via `tiny_skia_path::PathStroker::stroke`.
fn push_fill_and_stroke(path: &usvg::Path, out: &mut Vec<SvgPath>) {
    if path.fill().is_some()
        && let Some(segs) = path_segments(path.data())
    {
        out.push(SvgPath { segments: segs });
    }
    if let Some(stroke) = path.stroke()
        && let Some(stroked) = path.data().stroke(&stroke.to_tiny_skia(), 1.0)
        && let Some(segs) = path_segments(&stroked)
    {
        out.push(SvgPath { segments: segs });
    }
}

fn path_segments(path: &usvg::tiny_skia_path::Path) -> Option<Vec<PathSegment>> {
    let segs: Vec<PathSegment> = path.segments().collect();
    if segs.is_empty() { None } else { Some(segs) }
}
