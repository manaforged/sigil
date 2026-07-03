//! Small first-party widgets built from the public Sigil primitives.
//!
//! These are intentionally boring. They are the stable examples for
//! how app authors should compose layout, style, events, and reactive
//! state without reaching into the lower-level crates.

mod button;
mod checkbox;
mod chip;
mod code_edit;
mod composition;
mod divider;
mod dot;
mod dropdown;
mod floating_text;
mod icon;
pub mod layout_ext;
mod menu;
mod modal;
pub mod number_field;
mod panel;
pub mod patterns;
mod progress;
mod radio_group;
mod scroll;
mod shared;
mod slider;
mod splitter;
mod tabs;
mod text_edit;
mod text_field;
mod toggle;
mod tooltip;
pub mod transition_seam;
mod virtual_list;

pub use button::{ButtonBuilder, button};
pub use checkbox::{CheckboxBuilder, CheckboxTone, checkbox};
pub use chip::{ChipTone, chip};
pub use code_edit::{CodeEditBuilder, HlSpan, code_edit};
pub use composition::{kv_row, status_bar};
pub use divider::{horizontal as divider_horizontal, vertical as divider_vertical};
pub use dot::{DotBuilder, dot};
pub use dropdown::{DropdownBuilder, dropdown};
pub use floating_text::{
    FCT_POOL_CAPACITY, FctNode, FctSpawn, FctState, FloatingTextBuilder, Particle, floating_text,
};
pub use icon::{IconBuilder, icon};
pub use menu::{MenuBuilder, MenuItem, menu};
pub use modal::{ModalBuilder, modal};
pub use number_field::{NumberFieldBuilder, NumericKind, number_field};
pub use panel::{PanelBuilder, panel};
pub use patterns::HintOverlayBuilder;
pub use progress::{ProgressBuilder, progress};
pub use radio_group::{RadioGroupBuilder, radio_group};
pub use scroll::{ScrollBuilder, scroll};
pub use shared::label;
pub use slider::{Orientation, SliderBuilder, slider};
pub use splitter::{PaneBuilder, SplitterBuilder, SplitterOrientation, splitter};
pub use tabs::{TabsBuilder, tabs};
pub use text_edit::{TextEditBuilder, text_edit};
pub use text_field::{Clipboard, TextFieldBuilder, text_field};
pub use toggle::{ToggleBuilder, toggle};
pub use tooltip::{TooltipBuilder, tooltip};
pub use virtual_list::{
    ScrollAnchor, SelectionMode, SelectionState, VirtualListBuilder, virtual_list,
};
