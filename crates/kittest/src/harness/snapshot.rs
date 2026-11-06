use super::{Harness, HarnessSurface};
use crate::compare::assert_text_matches_golden;
use crate::gpu::{headless_renderer, render_scene_to_image};
use crate::render::physical_extent;
use crate::{Error, Golden, Rendered};

impl Harness {
    pub fn image(&mut self) -> Rendered {
        self.live();
        if let HarnessSurface::Cells { screen, size } = &mut self.surface {
            let grid = screen
                .frame(&mut self.host, [], *size)
                .map_err(|error| Error::Layout(error.to_string()))?;
            return grid
                .image(
                    self.host.theme(),
                    sigil::geometry::PhysicalSize::new(10, 20),
                )
                .map(Some)
                .map_err(|error| Error::Render(error.to_string()));
        }
        let (size, scale) = match &self.surface {
            HarnessSurface::Pixels { size, scale, .. } => (*size, *scale),
            HarnessSurface::Gpu(gpu) => return gpu.image().map(Some),
            HarnessSurface::Cells { .. } => {
                return Err(Error::Render(
                    "a cell harness has no pixel surface".to_owned(),
                ));
            }
        };
        let viewport = self.viewport();
        let mut scene = self.host.emit(viewport);
        sigil::scene::paint_backdrop(&mut scene, self.host.theme(), viewport);
        let Some(mut renderer) = headless_renderer()? else {
            return Ok(None);
        };
        let device = [size.0, size.1].map(|extent| physical_extent(extent, scale));
        render_scene_to_image(&mut renderer, &scene, viewport, device).map(Some)
    }

    #[track_caller]
    pub fn snapshot(&mut self, name: &str) {
        self.snapshot_as(name, &Golden::new(name));
    }

    /// Compares this surface with a golden at a pixel budget.
    #[track_caller]
    pub fn snapshot_with(&mut self, name: &str, max_delta_pct: f32) {
        self.snapshot_as(name, &Golden::new(name).delta(max_delta_pct));
    }

    #[track_caller]
    #[allow(clippy::print_stdout)]
    fn snapshot_as(&mut self, name: &str, golden: &Golden) {
        if matches!(self.live().surface, HarnessSurface::Cells { .. }) {
            let text = self.text();
            assert_text_matches_golden(&text, &golden.path().with_extension("txt"));
            return;
        }
        match self.image() {
            Ok(Some(image)) => golden.assert_image(&image),
            Ok(None) => println!("no GPU adapter — skipping snapshot {name}"),
            Err(err) => super::fail(format!("snapshot {name}: {err}")),
        }
    }
}
