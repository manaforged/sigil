//! Small first-party widgets built from the public Sigil primitives.
//!
//! These are intentionally boring. They are the stable examples for
//! how app authors should compose layout, style, events, and reactive
//! state without reaching into the lower-level crates.

mod button;
mod checkbox;
mod dropdown;
mod panel;
mod progress;
mod scroll;
mod shared;
mod slider;
mod text_edit;
mod text_field;
mod toggle;

pub use button::{ButtonBuilder, ButtonTone, button};
pub use checkbox::{CheckboxBuilder, CheckboxTone, checkbox};
pub use dropdown::{DropdownBuilder, dropdown};
pub use panel::{PanelBuilder, panel};
pub use progress::{ProgressBuilder, progress};
pub use scroll::{ScrollBuilder, scroll};
pub use shared::label;
pub use slider::{Orientation, SliderBuilder, slider};
pub use text_field::{Clipboard, TextFieldBuilder, text_field};
pub use toggle::{ToggleBuilder, toggle};
