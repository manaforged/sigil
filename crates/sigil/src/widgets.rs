//! Small first-party widgets built from the public Sigil primitives.
//!
//! These are intentionally boring. They are the stable examples for
//! how app authors should compose layout, style, events, and reactive
//! state without reaching into the lower-level crates.

mod button;
mod checkbox;
mod dropdown;
mod floating_text;
mod menu;
mod modal;
mod panel;
mod progress;
mod scroll;
mod shared;
mod slider;
mod tabs;
mod text_edit;
mod text_field;
mod toggle;
mod tooltip;
pub mod transition_seam;

pub use button::{ButtonBuilder, ButtonTone, button};
pub use checkbox::{CheckboxBuilder, CheckboxTone, checkbox};
pub use dropdown::{DropdownBuilder, dropdown};
pub use floating_text::{
    FCT_POOL_CAPACITY, FctNode, FctSpawn, FctState, FloatingTextBuilder, Particle, floating_text,
};
pub use menu::{MenuBuilder, MenuItem, menu};
pub use modal::{ModalBuilder, modal};
pub use panel::{PanelBuilder, panel};
pub use progress::{ProgressBuilder, progress};
pub use scroll::{ScrollBuilder, scroll};
pub use shared::label;
pub use slider::{Orientation, SliderBuilder, slider};
pub use tabs::{TabsBuilder, tabs};
pub use text_field::{Clipboard, TextFieldBuilder, text_field};
pub use toggle::{ToggleBuilder, toggle};
pub use tooltip::{TooltipBuilder, tooltip};
