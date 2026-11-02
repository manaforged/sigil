use crate::geometry::{PhysicalSize, Point, Rect};
use crate::renderer::CpuRenderer;
use crate::renderer::capture::{ImageRenderer, ImageTarget, render_to_image};
use crate::scene::{
    CommandScene, DecorationKind, DrawDecoration, DrawRect, GlyphRun, SceneCommand,
};
use crate::style::{Color, Rgba8, StyledTextEngine as _, TextLineHeight, TextStyle, Theme};
use crate::terminal::colour;
use crate::terminal::paint::{Cell, Grid, Underline};
use crate::text::OwnedTextEngine;
use crate::{Error, ErrorKind};

fn capture_size(grid: &Grid, cell: PhysicalSize) -> Result<PhysicalSize, Error> {
    let extent = |cells: usize, pixels: u32| {
        u32::try_from(cells)
            .ok()
            .and_then(|count| count.checked_mul(pixels))
    };
    let size = match (
        extent(grid.cols(), cell.width),
        extent(grid.rows(), cell.height),
    ) {
        (Some(width), Some(height)) if width > 0 && height > 0 => PhysicalSize::new(width, height),
        _ => {
            return Err(Error::fixed(
                ErrorKind::InvalidViewport,
                "viewport dimensions must be strictly positive (width > 0, height > 0)",
            ));
        }
    };
    if grid.images().is_empty() {
        Ok(size)
    } else {
        Err(Error::fixed(
            ErrorKind::Software,
            "software renderer failed: terminal graphics capture is not supported",
        ))
    }
}

impl Grid {
    /// Renders the grid to a PNG-ready image with this cell size.
    #[allow(
        clippy::cast_precision_loss,
        reason = "terminal image dimensions fit the f32 mantissa"
    )]
    pub fn image(&self, theme: &Theme, cell: PhysicalSize) -> Result<image::RgbaImage, Error> {
        let size = capture_size(self, cell)?;
        let mut scene = CommandScene::default();
        for y in 0..self.rows() {
            for (x, value) in self.row(y).iter().enumerate() {
                let rect = area(x, y, cell);
                let (_, bg) = inks(value, theme, self.cursor() == Some((x, y)));
                scene
                    .commands
                    .push(SceneCommand::Rect(DrawRect::new(rect, bg)));
            }
        }
        for y in 0..self.rows() {
            for (x, value) in self.row(y).iter().enumerate() {
                let (fg, _) = inks(value, theme, self.cursor() == Some((x, y)));
                paint(&mut scene, value, area(x, y, cell), fg)?;
            }
        }
        render_to_image(
            ImageRenderer::Cpu(&mut CpuRenderer::new()),
            &scene,
            ImageTarget {
                size,
                viewport: (size.width as f32, size.height as f32),
                clear: Color::rgba(0.0, 0.0, 0.0, 0.0),
            },
        )
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "terminal coordinates are u16 cell positions"
)]
fn area(x: usize, y: usize, cell: PhysicalSize) -> Rect {
    Rect::new(
        Point::new(x as f32 * cell.width as f32, y as f32 * cell.height as f32),
        cell.width as f32,
        cell.height as f32,
    )
}

fn inks(cell: &Cell, theme: &Theme, cursor: bool) -> (Rgba8, Rgba8) {
    let fg = colour::rgba(cell.fg, theme.on_surface);
    let bg = colour::rgba(cell.bg, theme.surface);
    if cursor { (bg, fg) } else { (fg, bg) }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "grapheme widths are small cell counts"
)]
fn paint(scene: &mut CommandScene, cell: &Cell, mut rect: Rect, fg: Rgba8) -> Result<(), Error> {
    let width = crate::app::cell_width(&cell.text);
    if width == 0 {
        return Ok(());
    }
    rect.width *= width as f32;
    if !cell.text.trim().is_empty() {
        let style = TextStyle {
            font_family: String::from("mono"),
            font_size: rect.height * 0.8,
            font_weight: if cell.bold { 700 } else { 400 },
            italic: cell.italic,
            line_height: TextLineHeight::Absolute(rect.height),
            ..TextStyle::default()
        };
        let paragraph = OwnedTextEngine::thread()
            .layout_paragraph_styled(&cell.text, None, &style)
            .map_err(|error| {
                Error::message(
                    ErrorKind::Software,
                    format_args!("software renderer failed: {error}"),
                )
            })?;
        let opacity = if cell.dim { 5_000 } else { 10_000 };
        let run = GlyphRun::new(rect, cell.text.to_string(), fg, style, opacity)
            .with_paragraph(paragraph);
        scene.commands.push(SceneCommand::PushClip(rect));
        scene.commands.push(SceneCommand::GlyphRun(run));
        scene.commands.push(SceneCommand::PopClip);
    }
    decorations(scene, cell, rect, fg);
    Ok(())
}

fn decorations(scene: &mut CommandScene, cell: &Cell, rect: Rect, fg: Rgba8) {
    let underline = match cell.lines.underline {
        Underline::None => None,
        Underline::Straight => Some(DecorationKind::Underline),
        Underline::Curly => Some(DecorationKind::WavyUnderline),
    };
    for kind in underline
        .into_iter()
        .chain(cell.lines.strike.then_some(DecorationKind::Strikethrough))
    {
        let y = if kind == DecorationKind::Strikethrough {
            0.5
        } else {
            0.9
        };
        scene
            .commands
            .push(SceneCommand::Decoration(DrawDecoration {
                node: None,
                kind,
                band: Rect::new(
                    Point::new(rect.origin.x, rect.height.mul_add(y, rect.origin.y)),
                    rect.width,
                    1.0,
                ),
                color: colour::rgba(cell.underline_ink, fg),
                opacity: if cell.dim { 5_000 } else { 10_000 },
                period: 4.0,
                line_width: 1.0,
            }));
    }
}

#[cfg(test)]
#[path = "capture_tests.rs"]
mod tests;
