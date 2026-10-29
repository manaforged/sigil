use super::{Harness, Surface};
use crate::gpu::{headless_renderer, render_scene_to_image};
use crate::{Error, Rendered, assert_png_matches_golden, assert_text_matches_golden};

pub const SNAPSHOT_MAX_DELTA_PCT: f32 = 0.5;

impl Harness {
    pub fn image(&mut self) -> Rendered {
        self.live();
        if let Surface::Cells { screen, size } = &mut self.surface {
            let grid = screen
                .frame(&mut self.host, &mut self.router, [], *size)
                .map_err(|error| Error::Layout(error.to_string()))?;
            return grid
                .image(
                    self.host.theme(),
                    sigil::geometry::PhysicalSize::new(10, 20),
                )
                .map(Some)
                .map_err(|error| Error::Render(error.to_string()));
        }
        let Surface::Pixels { size, .. } = self.surface else {
            return Err(Error::Render(
                "image needs a cell or pixel harness".to_owned(),
            ));
        };
        let viewport = self.viewport();
        let mut scene = self.host.emit(viewport);
        sigil::scene::paint_backdrop(&mut scene, self.host.theme(), viewport);
        let Some(mut renderer) = headless_renderer()? else {
            return Ok(None);
        };
        render_scene_to_image(&mut renderer, &scene, viewport, [size.0, size.1]).map(Some)
    }

    #[track_caller]
    pub fn snapshot(&mut self, name: &str) {
        self.snapshot_with(name, SNAPSHOT_MAX_DELTA_PCT);
    }

    #[track_caller]
    #[allow(clippy::print_stdout)]
    pub fn snapshot_with(&mut self, name: &str, max_delta_pct: f32) {
        let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_owned());
        let golden = crate::golden_path_in(&dir, name);
        if matches!(self.live().surface, Surface::Cells { .. }) {
            let text = self.text();
            assert_text_matches_golden(&text, &golden.with_extension("txt"));
            return;
        }
        match self.image() {
            Ok(Some(image)) => assert_png_matches_golden(&image, &golden, max_delta_pct),
            Ok(None) => println!("no GPU adapter — skipping snapshot {name}"),
            Err(err) => super::fail(format!("snapshot {name}: {err}")),
        }
    }
}
