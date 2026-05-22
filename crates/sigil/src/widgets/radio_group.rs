//! Mutually exclusive selection — N labelled options, one chosen.
//!
//! ```ignore
//! use sigil::prelude::*;
//!
//! let theme = signal(String::from("light"));
//! radio_group(theme)
//!     .option("light", "Light")
//!     .option("dark", "Dark")
//!     .option("auto", "Match system");
//! ```
//!
//! The selection signal holds the active option's key. Clicking a
//! row writes the row's key into the signal — any subscriber
//! observing the signal sees the new value. Keys and labels are
//! owned `String`s so runtime values (i18n labels, locale codes
//! loaded from disk, enum-derived keys) compose without
//! `Box::leak`.
//!
//! # V1 a11y caveat
//!
//! The widget advertises `Role::RadioGroup` + `Role::RadioButton`
//! and now wires the per-row `checked` signal so screen readers can
//! read which option is selected. **But** the rows are not yet
//! keyboard-operable: they're not in the focus ring, they have no
//! focus-ring atom, and they accept neither Space/Enter activation
//! nor arrow-key navigation. Keyboard-only users currently cannot
//! reach or change the selection through the focus router. Use at
//! pointer-driven call sites only until the keyboard pass lands.

use std::rc::Rc;

use events::prelude::NodeEventsExt;
use kernel::{NodeBuilder, Role, container, text};
use layout::prelude::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle};
use reactive::{Signal, derived};
use style::{NodeStyleExt, Style, StyleAtom, Tone};

use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use crate::prelude::{IntoNode, StyleAccess};

/// One radio option — a `(key, label)` pair.
#[derive(Clone, Debug)]
struct RadioOption {
    key: String,
    label: String,
}

/// Builder returned by [`radio_group`].
pub struct RadioGroupBuilder {
    selected: Signal<String>,
    options: Vec<RadioOption>,
    tone: Tone,
    style_accumulator: style::Style,
    layout_overrides: LayoutOverrides,
}

/// Create a radio group bound to the given selection signal.
#[must_use]
pub fn radio_group(selected: Signal<String>) -> RadioGroupBuilder {
    RadioGroupBuilder::new(selected)
}

impl RadioGroupBuilder {
    /// Construct an empty radio group bound to `selected`.
    #[must_use]
    pub fn new(selected: Signal<String>) -> Self {
        Self {
            selected,
            options: Vec::new(),
            tone: Tone::Primary,
            style_accumulator: style::Style::new(),
            layout_overrides: LayoutOverrides::new(),
        }
    }

    /// Add an option. `key` is the value stored in the selection
    /// signal when this option is chosen; `label` is the visible
    /// text. Both accept `&str`, `String`, or anything that converts
    /// `Into<String>`.
    #[must_use]
    pub fn option(mut self, key: impl Into<String>, label: impl Into<String>) -> Self {
        self.options.push(RadioOption {
            key: key.into(),
            label: label.into(),
        });
        self
    }

    /// Set the accent tone used for the selected option's mark.
    #[must_use]
    pub const fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let selected = self.selected;
        let tone = self.tone;
        let mut group = container()
            .role(Role::RadioGroup)
            .layout(self.layout_overrides.apply(group_layout()));
        for opt in self.options {
            group = group.child(option_row(opt, selected, tone));
        }
        if !self.style_accumulator.is_empty() {
            group = group.style(self.style_accumulator);
        }
        group
    }
}

fn option_row(opt: RadioOption, selected: Signal<String>, tone: Tone) -> NodeBuilder {
    let RadioOption { key, label } = opt;
    let key_for_activate = key.clone();
    let activate = Rc::new(move || selected.set(key_for_activate.clone()));
    let activate_for_pointer = Rc::clone(&activate);
    // Per-row `checked` signal — derives from the group's selected
    // string. Without this the row's `Role::RadioButton` advertises a
    // selectable contract that assistive tech can't actually read
    // (caught by adversarial review 2026-05-21).
    let key_for_checked = key.clone();
    let checked: Signal<bool> = derived(move || selected.get() == key_for_checked);
    container()
        .role(Role::RadioButton)
        .label(label.clone())
        .checked(checked)
        .layout(option_layout())
        .class("rounded-sm")
        .tone(tone)
        .child(option_mark(selected, key, tone))
        .child(text(label).class("text-on-surface"))
        .pointer_up(move |_event| {
            activate_for_pointer();
        })
}

fn option_mark(selected: Signal<String>, key: String, tone: Tone) -> NodeBuilder {
    // Inner dot — TONAL fill (resolves through the tone's scale at
    // step 9 when selected). The tone context lives on this node so
    // the FILL gets the accent color while the OUTER ring stays
    // neutral.
    let mark_inner = container()
        .layout(mark_inner_layout())
        .tone(tone)
        // MARK_INNER is 8 px; explicit radius avoids the Px(9_999) SDF clipping bug
        // where rounded-full on a small rect evaluates every pixel as outside the shape.
        .style(Style::new().radius(4))
        .style_reactive(move || {
            if selected.get() == key {
                MARK_SELECTED
            } else {
                MARK_EMPTY
            }
        });
    // Outer frame — NEUTRAL border. Borders are neutral by design
    // (step-7 of the gray scale = Radix "non-text borders" at
    // ≥3:1 against surface). Earlier passes attached the widget's
    // tone here too, which made the frame pale-lavender on iris
    // and failed the 3:1 floor on light surfaces (the reviewer visual
    // adversarial 2026-05-21). The `bg-gray-7` class resolves
    // through the gray scale directly, ignoring the tone context.
    container()
        .role(Role::GenericContainer)
        .layout(mark_outer_layout())
        // MARK_OUTER is 16 px; explicit radius avoids the Px(9_999) SDF clipping bug.
        .style(Style::new().radius(8))
        .class("bg-gray-7")
        .child(mark_inner)
}

// Inner-fill atoms — read on the tonal inner node, so step-9
// resolves through the accent (iris / success / error / etc.)
// scale. MARK_OUTER_FRAME is no longer needed; the outer ring's
// neutral border is supplied via the `bg-gray-7` class above.
static MARK_SELECTED: &[StyleAtom] = &[StyleAtom::BgToneStep(9)];
static MARK_EMPTY: &[StyleAtom] = &[];

const ROW_HEIGHT: f32 = 28.0;
const MARK_OUTER: f32 = 16.0;
const MARK_INNER: f32 = 8.0;

fn group_layout() -> LayoutStyle {
    LayoutStyle::flex_column().gap(4.0)
}

fn option_layout() -> LayoutStyle {
    LayoutStyle::flex_row()
        .height(ROW_HEIGHT)
        .padding_x(4.0)
        .gap(8.0)
        .align_items(AlignItems::Center)
}

fn mark_outer_layout() -> LayoutStyle {
    LayoutStyle::flex_row()
        .fixed_size(MARK_OUTER, MARK_OUTER)
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::Center)
}

fn mark_inner_layout() -> LayoutStyle {
    LayoutStyle::flex_row().fixed_size(MARK_INNER, MARK_INNER)
}

impl StyleAccess for RadioGroupBuilder {
    fn style_mut(&mut self) -> &mut style::Style {
        &mut self.style_accumulator
    }
}

impl WidgetLayoutExt for RadioGroupBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl IntoNode for RadioGroupBuilder {
    fn build(self, cx: &mut kernel::Cx) -> kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use kernel::Cx;
    use reactive::{Runtime, signal};

    #[test]
    fn group_carries_radio_group_role() {
        Runtime::scope("radio-group-role", || {
            let mut cx = Cx::new_root();
            let selected = signal(String::from("a"));
            let id = radio_group(selected)
                .option("a", "A")
                .option("b", "B")
                .build(&mut cx);
            let node = cx.get(id).expect("group node");
            assert_eq!(node.a11y.role, kernel::Role::RadioGroup);
            assert_eq!(node.children().len(), 2, "two option rows");
        });
    }

    #[test]
    fn option_rows_carry_radio_button_role() {
        Runtime::scope("radio-group-option-role", || {
            let mut cx = Cx::new_root();
            let selected = signal(String::from("a"));
            let id = radio_group(selected).option("a", "A").build(&mut cx);
            let group = cx.get(id).expect("group node");
            let row = cx.get(group.children()[0]).expect("row node");
            assert_eq!(row.a11y.role, kernel::Role::RadioButton);
        });
    }

    #[test]
    fn defaults_to_primary_tone() {
        // The default tone is `Tone::Primary` so the selected mark
        // paints with the accent color out of the box. Changing the
        // default would silently re-tint every existing call site.
        Runtime::scope("radio-group-default-tone", || {
            let group = radio_group(signal(String::from("a"))).option("a", "A");
            assert!(matches!(group.tone, Tone::Primary));
        });
    }
}
