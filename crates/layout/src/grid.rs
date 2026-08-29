#![allow(clippy::indexing_slicing, clippy::cast_precision_loss)]

use ahash::AHashSet;

use crate::LayoutError;
use crate::block::{BlockSolver, Position2};
use crate::flex::{CbSize, FlexSolver};
use crate::forest::{ComputedBox, LocalId, MeasureScope, NodeRef, SpaceConstraint};
use crate::resolve::NodeEdges;
use crate::style_data::{Dimension, Display, LengthPercentage, Size, StyleData, TrackSize};

pub(crate) struct GridSolver<'a> {
    scope: MeasureScope<'a>,
}

struct Cols {
    widths: Vec<f32>,
    x: Vec<f32>,
    n: usize,
}

impl Cols {
    fn new(content_x: f32, widths: Vec<f32>, gap_x: f32) -> Self {
        Self {
            n: widths.len().max(1),
            x: offsets(content_x, &widths, gap_x),
            widths,
        }
    }
}

struct RowTracks<'a> {
    y: &'a [f32],
    h: &'a [f32],
    gap: f32,
}

impl<'a> RowTracks<'a> {
    const fn new(y: &'a [f32], h: &'a [f32], gap: f32) -> Self {
        Self { y, h, gap }
    }
}

struct GridFrame<'a> {
    style: &'a StyleData,
    edges: NodeEdges,
    gap_y: f32,
    cols: Cols,
    border: Position2,
    content: Position2,
}

struct Placement {
    cid: LocalId,
    row: usize,
    col: usize,
    span: usize,
    row_span: usize,
}

struct Spec {
    cid: LocalId,
    span: usize,
    row_span: usize,
    at: Option<(usize, usize)>,
}

fn reserve(
    occupied: &mut AHashSet<(usize, usize)>,
    r: usize,
    c: usize,
    cspan: usize,
    rspan: usize,
) {
    for dr in 0..rspan {
        for dc in 0..cspan {
            occupied.insert((r + dr, c + dc));
        }
    }
}

fn free_cell(
    occupied: &AHashSet<(usize, usize)>,
    start: (usize, usize),
    ncols: usize,
    cspan: usize,
    rspan: usize,
) -> (usize, usize) {
    let (mut r, mut c) = start;
    loop {
        if c + cspan > ncols {
            r += 1;
            c = 0;
            continue;
        }
        let fits = (0..rspan).all(|dr| (0..cspan).all(|dc| !occupied.contains(&(r + dr, c + dc))));
        if fits {
            return (r, c);
        }
        c += 1;
    }
}

fn spanned_width(cols: &Cols, col: usize, span: usize) -> f32 {
    let last = col + span - 1;
    cols.x[last] + cols.widths[last] - cols.x[col]
}

impl<'a> GridSolver<'a> {
    pub(crate) const fn new(scope: MeasureScope<'a>) -> Self {
        Self { scope }
    }

    pub(crate) fn solve(&self, subtree: NodeRef<'_>) -> Result<(), LayoutError> {
        let avail = self.scope.available();
        let border_w = match subtree.style().size.width {
            Dimension::Px(px) => px,
            _ => definite(avail.w),
        };
        self.solve_at(subtree, border_w, Position2 { x: 0.0, y: 0.0 })
    }

    pub(crate) fn solve_at(
        &self,
        subtree: NodeRef<'_>,
        border_w: f32,
        origin: Position2,
    ) -> Result<(), LayoutError> {
        let style = subtree.style();
        let edges = NodeEdges::build(style, border_w);
        let inner_w = (border_w - edges.inline_chrome()).max(0.0);
        let gap_x = resolve_gap(style.gap.width, inner_w);
        let gap_y = resolve_gap(style.gap.height, inner_w);
        let widths = resolve_columns(&style.grid_template_columns, inner_w, gap_x);
        let border = Position2 {
            x: origin.x + edges.margin.left.unwrap_or(0.0),
            y: origin.y + edges.margin_top(),
        };
        let content = Position2 {
            x: border.x + edges.padding.left + edges.border.left,
            y: border.y + edges.padding.top + edges.border.top,
        };
        let frame = GridFrame {
            style,
            edges,
            gap_y,
            cols: Cols::new(content.x, widths, gap_x),
            border,
            content,
        };
        let children: Vec<LocalId> = subtree.children().to_vec();
        let places = self.flow_children(&children, frame.cols.n, frame.style.grid_auto_flow_dense);
        let row_h = self.layout_rows(&places, &frame)?;

        let avail_h = definite(self.scope.available().h);
        let content_h = frame.gap_y.mul_add(
            (row_h.len().saturating_sub(1)) as f32,
            row_h.iter().sum::<f32>(),
        );
        let box_h = grid_box_h(frame.style, avail_h, content_h, frame.edges.block_chrome());
        self.scope.write_node(
            &subtree,
            ComputedBox {
                x: frame.border.x,
                y: frame.border.y,
                w: border_w,
                h: box_h,
            },
        );
        Ok(())
    }

    fn layout_rows(
        &self,
        places: &[Placement],
        frame: &GridFrame<'_>,
    ) -> Result<Vec<f32>, LayoutError> {
        let mut row_h = self.measure_row_heights(places, &frame.cols)?;
        apply_row_template(&mut row_h, &frame.style.grid_template_rows);
        let avail_h = definite(self.scope.available().h);
        let chrome = frame.edges.block_chrome();
        let inner_h = match frame.style.size.height {
            Dimension::Px(px) => Some((px - chrome).max(0.0)),
            Dimension::Fill => Some((avail_h - chrome).max(0.0)),
            Dimension::Percent(pct) => Some(avail_h.mul_add(pct, -chrome).max(0.0)),
            Dimension::Auto if avail_h > 0.0 => Some((avail_h - chrome).max(0.0)),
            Dimension::Auto => None,
        };
        if let Some(height) = inner_h {
            distribute_fr_rows(
                &mut row_h,
                &frame.style.grid_template_rows,
                height,
                frame.gap_y,
            );
        }
        let row_y = offsets(frame.content.y, &row_h, frame.gap_y);
        self.place_cells(
            places,
            &frame.cols,
            &RowTracks::new(&row_y, &row_h, frame.gap_y),
        )?;
        Ok(row_h)
    }

    fn flow_children(&self, children: &[LocalId], ncols: usize, dense: bool) -> Vec<Placement> {
        let specs: Vec<Spec> = children
            .iter()
            .map(|&c| self.child_spec(c, ncols))
            .collect();
        let mut occupied: AHashSet<(usize, usize)> = AHashSet::new();
        for s in &specs {
            if let Some((r, c)) = s.at {
                reserve(&mut occupied, r, c, s.span, s.row_span);
            }
        }
        let mut out = Vec::with_capacity(specs.len());
        let (mut row, mut col) = (0usize, 0usize);
        for s in &specs {
            let (r, c) = s.at.unwrap_or_else(|| {
                let start = if dense { (0, 0) } else { (row, col) };
                let cell = free_cell(&occupied, start, ncols, s.span, s.row_span);
                reserve(&mut occupied, cell.0, cell.1, s.span, s.row_span);
                if !dense {
                    (row, col) = (cell.0, cell.1 + s.span);
                }
                cell
            });
            out.push(Placement {
                cid: s.cid,
                row: r,
                col: c,
                span: s.span,
                row_span: s.row_span,
            });
        }
        out
    }

    fn child_spec(&self, cid: LocalId, ncols: usize) -> Spec {
        let (span, row_span, cstart, rstart) =
            self.scope.forest().refer(cid).map_or((1, 1, 0, 0), |n| {
                let s = n.style();
                (
                    (s.grid_column_span as usize).clamp(1, ncols),
                    (s.grid_row_span as usize).max(1),
                    s.grid_column_start as usize,
                    s.grid_row_start as usize,
                )
            });
        let at = (cstart > 0 && rstart > 0)
            .then(|| ((rstart - 1), (cstart - 1).min(ncols.saturating_sub(span))));
        Spec {
            cid,
            span,
            row_span,
            at,
        }
    }

    fn measure_row_heights(
        &self,
        places: &[Placement],
        cols: &Cols,
    ) -> Result<Vec<f32>, LayoutError> {
        let nrows = places.iter().map(|p| p.row + p.row_span).max().unwrap_or(0);
        let mut row_h = vec![0.0_f32; nrows];
        for p in places {
            self.layout_cell(
                p.cid,
                spanned_width(cols, p.col, p.span),
                0.0,
                Position2 { x: 0.0, y: 0.0 },
            )?;
            let h = self.scope.forest().boxed(p.cid).map_or(0.0, |b| b.h);
            let per_row = h / p.row_span as f32;
            for dr in 0..p.row_span {
                row_h[p.row + dr] = row_h[p.row + dr].max(per_row);
            }
        }
        Ok(row_h)
    }

    fn place_cells(
        &self,
        places: &[Placement],
        cols: &Cols,
        rows: &RowTracks<'_>,
    ) -> Result<(), LayoutError> {
        for p in places {
            let origin = Position2 {
                x: cols.x[p.col],
                y: rows.y[p.row],
            };
            let w = spanned_width(cols, p.col, p.span);
            let h = spanned_height(rows.h, p.row, p.row_span, rows.gap);
            self.layout_cell(p.cid, w, h, origin)?;
        }
        Ok(())
    }

    fn layout_cell(
        &self,
        cid: LocalId,
        w: f32,
        h: f32,
        origin: Position2,
    ) -> Result<(), LayoutError> {
        let forest = self.scope.forest();
        let cell = forest
            .refer(cid)
            .ok_or_else(|| LayoutError::LayoutInvalid(format!("stale grid cell {cid:?}")))?;
        let stretch = matches!(cell.style().size.height, Dimension::Auto);
        let nested = MeasureScope::new(forest, self.scope.text(), self.scope.available());
        match cell.style().display {
            Display::Flex => FlexSolver::new(nested).solve_at_area(
                cell,
                CbSize { w, h },
                crate::flex::Position2 {
                    x: origin.x,
                    y: origin.y,
                },
            ),
            Display::Grid => GridSolver::new(nested).solve_at(cell, w, origin),
            Display::Block | Display::None => BlockSolver::new(nested).solve_at_assigned(
                cell,
                Size {
                    width: w,
                    height: h,
                },
                origin,
                Some(w),
            ),
        }?;
        if stretch && h > 0.0 {
            self.stretch_box(cid, h);
        }
        Ok(())
    }

    fn stretch_box(&self, cid: LocalId, track_h: f32) {
        let Some(current) = self.scope.forest().boxed(cid) else {
            return;
        };
        if current.h + 0.5 >= track_h {
            return;
        }
        let Some(node) = self.scope.forest().refer(cid) else {
            return;
        };
        self.scope.write_node(
            &node,
            ComputedBox {
                x: current.x,
                y: current.y,
                w: current.w,
                h: track_h,
            },
        );
    }
}

fn grid_box_h(style: &StyleData, avail_h: f32, content_h: f32, chrome: f32) -> f32 {
    match style.size.height {
        Dimension::Px(px) => px,
        Dimension::Fill => avail_h,
        Dimension::Percent(pct) => avail_h * pct,
        Dimension::Auto if avail_h > 0.0 => avail_h,
        Dimension::Auto => content_h + chrome,
    }
}

fn spanned_height(row_h: &[f32], row: usize, span: usize, gap: f32) -> f32 {
    let end = (row + span).min(row_h.len());
    if row >= end {
        return 0.0;
    }
    let tracks = &row_h[row..end];
    gap.mul_add(
        (tracks.len().saturating_sub(1)) as f32,
        tracks.iter().sum::<f32>(),
    )
}

fn offsets(start: f32, sizes: &[f32], gap: f32) -> Vec<f32> {
    let mut out = Vec::with_capacity(sizes.len());
    let mut pos = start;
    for size in sizes {
        out.push(pos);
        pos += size + gap;
    }
    out
}

fn resolve_columns(tracks: &[TrackSize], inner_w: f32, gap_x: f32) -> Vec<f32> {
    if tracks.is_empty() {
        return vec![inner_w.max(0.0)];
    }
    let total_gap = gap_x * (tracks.len().saturating_sub(1) as f32);
    let mut sizes: Vec<Option<f32>> = tracks
        .iter()
        .map(|t| match t {
            TrackSize::Px(px) => Some(px.max(0.0)),
            _ => None,
        })
        .collect();
    loop {
        let fixed: f32 = sizes.iter().flatten().sum();
        let free = (inner_w - total_gap - fixed).max(0.0);
        let active_weight: f32 = tracks
            .iter()
            .zip(&sizes)
            .filter(|(_, s)| s.is_none())
            .map(|(t, _)| flex_weight(t))
            .sum();
        let per_fr = if active_weight > 0.0 {
            free / active_weight
        } else {
            0.0
        };
        let mut froze = false;
        for (t, s) in tracks.iter().zip(sizes.iter_mut()) {
            if s.is_none()
                && let TrackSize::MinMax(min, fr) = t
                && per_fr * fr.max(0.0) < min.max(0.0)
            {
                *s = Some(min.max(0.0));
                froze = true;
                break;
            }
        }
        if froze {
            continue;
        }
        for (t, s) in tracks.iter().zip(sizes.iter_mut()) {
            if s.is_none() {
                *s = Some(per_fr * flex_weight(t));
            }
        }
        break;
    }
    sizes.into_iter().map(|s| s.unwrap_or(0.0)).collect()
}

const fn flex_weight(track: &TrackSize) -> f32 {
    match track {
        TrackSize::Fr(fr) | TrackSize::MinMax(_, fr) => fr.max(0.0),
        TrackSize::Auto => 1.0,
        TrackSize::Px(_) => 0.0,
    }
}

fn distribute_fr_rows(row_h: &mut [f32], tracks: &[TrackSize], inner_h: f32, gap: f32) {
    if row_h.is_empty() {
        return;
    }
    let gaps = gap * (row_h.len().saturating_sub(1) as f32);
    let used: f32 = row_h.iter().sum::<f32>() + gaps;
    let leftover = inner_h - used;
    if leftover <= 0.0 {
        return;
    }
    let weight: f32 = (0..row_h.len())
        .map(|index| tracks.get(index).map_or(0.0, flex_weight))
        .sum();
    if weight <= 0.0 {
        return;
    }
    for (index, height) in row_h.iter_mut().enumerate() {
        let share = tracks.get(index).map_or(0.0, flex_weight);
        if share > 0.0 {
            *height = leftover.mul_add(share / weight, *height);
        }
    }
}

fn apply_row_template(row_h: &mut [f32], tracks: &[TrackSize]) {
    for (row, h) in row_h.iter_mut().enumerate() {
        if let Some(TrackSize::Px(px)) = tracks.get(row) {
            *h = px.max(0.0);
        }
    }
}

fn resolve_gap(gap: LengthPercentage, inner_w: f32) -> f32 {
    match gap {
        LengthPercentage::Px(px) => px.max(0.0),
        LengthPercentage::Percent(pct) => (pct * inner_w).max(0.0),
    }
}

const fn definite(c: SpaceConstraint) -> f32 {
    match c {
        SpaceConstraint::Definite(v) => v,
        SpaceConstraint::MinContent | SpaceConstraint::MaxContent => 0.0,
    }
}
