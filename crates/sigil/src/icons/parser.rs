use usvg::tiny_skia_path::PathSegment;

use crate::icons::BakeError;

pub(crate) struct SvgPath {
    pub(crate) segments: Vec<PathSegment>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ViewBox {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

pub(crate) fn svg_options() -> usvg::Options<'static> {
    usvg::Options {
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_data: Box::new(|_, _, _| None),
            resolve_string: Box::new(|_, _| None),
        },
        ..usvg::Options::default()
    }
}

pub(crate) fn parse(svg_source: &str) -> Result<(Vec<SvgPath>, ViewBox), BakeError> {
    if crate::widgets::source_too_large(svg_source) {
        return Err(BakeError::SvgParseError(crate::widgets::oversize_message()));
    }
    if nesting_depth(svg_source.as_bytes()) > MAX_ELEMENT_DEPTH {
        return Err(BakeError::SvgParseError(String::from(
            "SVG element nesting is deeper than the 128 limit",
        )));
    }
    let opts = svg_options();
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

const MAX_ELEMENT_DEPTH: usize = 128;

fn nesting_depth(source: &[u8]) -> usize {
    let mut depth = 0_usize;
    let mut max_depth = 0_usize;
    let mut index = 0;
    while let Some(offset) = source
        .get(index..)
        .and_then(|rest| rest.iter().position(|&b| b == b'<'))
    {
        let start = index + offset;
        let rest = source.get(start..).unwrap_or_default();
        if rest.starts_with(b"<!--") {
            index = skip_past(source, start, b"-->");
        } else if rest.starts_with(b"<![CDATA[") {
            index = skip_past(source, start, b"]]>");
        } else if rest.starts_with(b"<?") {
            index = skip_past(source, start, b"?>");
        } else if rest.starts_with(b"</") {
            depth = depth.saturating_sub(1);
            index = tag_end(source, start).0;
        } else if rest.starts_with(b"<!") {
            index = tag_end(source, start).0;
        } else {
            let (end, self_closing) = tag_end(source, start);
            if !self_closing {
                depth += 1;
                max_depth = max_depth.max(depth);
            }
            index = end;
        }
    }
    max_depth
}

fn skip_past(source: &[u8], start: usize, marker: &[u8]) -> usize {
    source
        .get(start..)
        .and_then(|rest| {
            rest.windows(marker.len())
                .position(|window| window == marker)
        })
        .map_or(source.len(), |offset| start + offset + marker.len())
}

fn tag_end(source: &[u8], start: usize) -> (usize, bool) {
    let mut quote = None;
    let mut previous = 0_u8;
    for (offset, &byte) in source.iter().enumerate().skip(start + 1) {
        match quote {
            Some(open) if byte == open => quote = None,
            None if byte == b'"' || byte == b'\'' => quote = Some(byte),
            None if byte == b'>' => return (offset + 1, previous == b'/'),
            _ => {}
        }
        previous = byte;
    }
    (source.len(), false)
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
