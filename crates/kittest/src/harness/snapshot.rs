use std::path::PathBuf;

use super::{Harness, Surface};
use crate::gpu::{headless_renderer, render_scene_to_image};
use crate::{Error, Rendered, assert_png_matches_golden, assert_text_matches_golden};

pub const SNAPSHOT_MAX_DELTA_PCT: f32 = 0.5;

fn manifest_dir() -> String {
    std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_owned())
}

impl Harness {
    pub fn image(&mut self) -> Rendered {
        let Surface::Pixels { size, .. } = self.live().surface else {
            return Err(Error::Render(
                "image needs a pixel harness; call .pixels(width, height)".to_owned(),
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
        let dir = manifest_dir();
        if matches!(self.live().surface, Surface::Cells { .. }) {
            let text = self.text();
            assert_text_matches_golden(&text, &text_golden(&dir, name));
            return;
        }
        match self.image() {
            Ok(Some(image)) => {
                assert_png_matches_golden(
                    &image,
                    &crate::golden_path_in(&dir, name),
                    max_delta_pct,
                );
            }
            Ok(None) => println!("no GPU adapter — skipping snapshot {name}"),
            Err(err) => super::fail(format!("snapshot {name}: {err}")),
        }
    }
}

fn text_golden(dir: &str, name: &str) -> PathBuf {
    crate::golden_path_in(dir, name).with_extension("txt")
}
