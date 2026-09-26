#![allow(clippy::indexing_slicing, clippy::cast_precision_loss)]

use ahash::AHashSet;

use crate::layout::LayoutError;
use crate::layout::block::{BlockSolver, Position2};
use crate::layout::flex::{CbSize, FlexSolver};
use crate::layout::forest::{ComputedBox, LocalId, MeasureScope, NodeRef};
use crate::layout::grid_tracks::{
    apply_row_template, definite, distribute_fr_rows, grid_box_h, offsets, resolve_columns,
    resolve_gap, spanned_height,
};
use crate::layout::resolve::NodeEdges;
use crate::layout::style_data::{Dimension, Display, Size, StyleData};

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
        let gap_x = resolve_gap(style.gap.width, inner_w, crate::layout::snap::gap);
        let gap_y = resolve_gap(style.gap.height, inner_w, crate::layout::snap::row_gap);
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
        let places = self.flow_children(&children, &frame);
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
            Dimension::Text(length) => Some((length.px() - chrome).max(0.0)),
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

    fn flow_children(&self, children: &[LocalId], frame: &GridFrame<'_>) -> Vec<Placement> {
        let (ncols, dense) = (frame.cols.n, frame.style.grid_auto_flow_dense);
        let row_cap = frame
            .style
            .grid_template_rows
            .len()
            .max(children.len())
            .clamp(1, MAX_ROW_SPAN);
        let specs: Vec<Spec> = children
            .iter()
            .map(|&c| self.child_spec(c, ncols, row_cap))
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

    fn child_spec(&self, cid: LocalId, ncols: usize, row_cap: usize) -> Spec {
        let (span, row_span, cstart, rstart) =
            self.scope.forest().refer(cid).map_or((1, 1, 0, 0), |n| {
                let s = n.style();
                (
                    (s.grid_column_span as usize).clamp(1, ncols),
                    (s.grid_row_span as usize).clamp(1, row_cap),
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
                crate::layout::flex::Position2 {
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
                Size {
                    width: Some(w),
                    height: None,
                },
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

const MAX_ROW_SPAN: usize = 1024;
