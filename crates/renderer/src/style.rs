//! Scene draw-style types under the `sigil_renderer::style` namespace.
//!
//! These are re-exports of the canonical definitions in `scene_command` under
//! shorter names. The old `Scene*Style` names remain as type aliases for one
//! release cycle; prefer these short names when writing new code.

pub use crate::scene_command::SceneDecorationStyle as Decoration;
pub use crate::scene_command::SceneGradientStyle as Gradient;
pub use crate::scene_command::ScenePatternStyle as Pattern;
pub use crate::scene_command::SceneRectStyle as Rect;
pub use crate::scene_command::SceneSpriteStyle as Sprite;
