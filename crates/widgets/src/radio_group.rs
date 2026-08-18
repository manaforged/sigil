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
//! # Keyboard + a11y
//!
//! The widget advertises `Role::RadioGroup` + `Role::RadioButton`
//! with per-row `checked` signals, and is fully keyboard-operable
//! (5/5 Wave 3 T10): the group is focusable; `↓` / `↑` move the
//! selection with wraparound, `Home` / `End` jump to the edges, and
//! the checked row shows the focus ring while the group holds focus
//! (ARIA roving-selection behavior — selection follows movement).

use std::rc::Rc;

use sigil_events::prelude::{Key, NodeEventsExt};
use sigil_kernel::{NodeBuilder, Role, container, text};
use sigil_layout::prelude::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle};
use sigil_reactive::{Signal, derived};
use sigil_style::{NodeStyleExt, Style, StyleAtom, Tone};

use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use sigil_kernel::IntoNode;
use sigil_style::StyleAccess;

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
    style_accumulator: sigil_style::Style,
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
            style_accumulator: sigil_style::Style::new(),
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
        let keys: Vec<String> = self.options.iter().map(|opt| opt.key.clone()).collect();
        // Roving-selection keyboard model (5/5 Wave 3 T10): the group is
        // focusable; Arrow/Home/End move the SELECTION itself (ARIA radio
        // behavior — selection follows movement), and the checked row
        // shows the focus ring while the group holds focus.
        let group_focus = sigil_reactive::signal(false);
        let mut group = container()
            .role(Role::RadioGroup)
            .focusable(true)
            .layout(self.layout_overrides.apply(group_layout()))
            .focus_in(move || group_focus.set(true))
            .focus_out(move || group_focus.set(false));
        for opt in self.options {
            group = group.child(option_row(opt, selected, tone, group_focus));
        }
        if !self.style_accumulator.is_empty() {
            group = group.style(self.style_accumulator);
        }
        let next_keys = keys.clone();
        let prev_keys = keys.clone();
        let home_keys = keys.clone();
        let end_keys = keys;
        group
            .key(Key::ArrowDown, move |event| {
                select_offset(selected, &next_keys, 1);
                event.stop_propagation();
            })
            .key(Key::ArrowUp, move |event| {
                select_offset(selected, &prev_keys, -1);
                event.stop_propagation();
            })
            .key(Key::Home, move |event| {
                select_edge(selected, &home_keys, true);
                event.stop_propagation();
            })
            .key(Key::End, move |event| {
                select_edge(selected, &end_keys, false);
                event.stop_propagation();
            })
    }
}

/// Move the selection `delta` steps from the currently selected key,
/// wrapping at both ends (ARIA radio navigation). No-ops on an empty
/// option list.
fn select_offset(selected: Signal<String>, keys: &[String], delta: isize) {
    if keys.is_empty() {
        return;
    }
    let current = selected.get();
    let index = keys
        .iter()
        .position(|key| key == &current)
        .map_or(0, usize::cast_signed);
    let len = keys.len().cast_signed();
    let next = (index + delta).rem_euclid(len).cast_unsigned();
    if let Some(key) = keys.get(next) {
        selected.set(key.clone());
    }
}

/// Select the first (`first == true`) or last option. No-ops when empty.
fn select_edge(selected: Signal<String>, keys: &[String], first: bool) {
    let key = if first { keys.first() } else { keys.last() };
    if let Some(key) = key {
        selected.set(key.clone());
    }
}

fn option_row(
    opt: RadioOption,
    selected: Signal<String>,
    tone: Tone,
    group_focus: Signal<bool>,
) -> NodeBuilder {
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
    // Virtual-focus ring: visible while the group holds focus AND this
    // row is the checked one (selection follows movement, so the ring
    // is always on the selected row).
    let key_for_ring = key.clone();
    let ring_visible = derived(move || group_focus.get() && selected.get() == key_for_ring);
    container()
        .role(Role::RadioButton)
        .label(label.clone())
        .checked(checked)
        .layout(option_layout())
        .class("rounded-sm")
        .tone(tone)
        .focus_ring(sigil_style::FocusRing::new(ring_visible))
        .child(option_mark(selected, key, tone))
        .child(text(label).class("text-on-surface"))
        .pointer_up(move |_event| {
            activate_for_pointer();
        })
}

fn option_mark(selected: Signal<String>, key: String, tone: Tone) -> NodeBuilder {
    // Inner dot — TONAL fill (resolves through the tone's scale at
    // step 8 when selected). The tone context lives on this node so
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
    // Surface core — punches the interior out of the outer disc so
    // the frame reads as a 1.5 px ring (the nested-fill border idiom;
    // text_field's frame is the precedent — no border atom exists).
    // ponytail: core paints the global `surface` token, so a radio on
    // a tinted panel shows a surface-colored hole; correct on every
    // current call site (all bg-surface). Upgrade path: SDF ring /
    // stroke primitive in the renderer (followup 2026-07-13).
    let mark_core = container()
        .layout(mark_core_layout())
        // MARK_CORE is 13 px; explicit radius avoids the Px(9_999) SDF clipping bug.
        .style(Style::new().radius(7))
        .class("bg-surface")
        .child(mark_inner);
    // Outer ring — NEUTRAL border. `gray-9` is the sanctioned rest-
    // border step (text_field frame, the reviewer visual 2026-05-21): the
    // previous flat `gray-7` fill measured 1.49:1 against the gray-2
    // surface, failing the 3:1 non-text floor this comment used to
    // claim it met (Fable visual adversarial 2026-07-13); gray-9
    // measures 3.14:1. The class resolves through the gray scale
    // directly, ignoring the tone context;
    // `mark_ring_clears_non_text_contrast` pins the floor.
    container()
        .role(Role::GenericContainer)
        .layout(mark_outer_layout())
        // MARK_OUTER is 16 px; explicit radius avoids the Px(9_999) SDF clipping bug.
        .style(Style::new().radius(8))
        .class("bg-gray-9")
        .child(mark_core)
}

// Inner-fill atoms — read on the tonal inner node, so the step
// resolves through the accent (steel-blue / success / error / etc.)
// scale. Step 8, not 9: the selected dot uses the muted state-fill
// register (same discipline as the toggle on-state); step 9 is
// reserved for primary actions. The outer ring's neutral border is
// supplied via the `bg-gray-9` class above.
static MARK_SELECTED: &[StyleAtom] = &[StyleAtom::BgToneStep(8)];
static MARK_EMPTY: &[StyleAtom] = &[];

const ROW_HEIGHT: f32 = 28.0;
const MARK_OUTER: f32 = 16.0;
// Core 13 px inside a 16 px ring = a 1.5 px stroke — the previous
// 2.5 px read clunky next to ShadCN's ~1 px rings (Fable 2026-08-15).
const MARK_CORE: f32 = 13.0;
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

fn mark_core_layout() -> LayoutStyle {
    LayoutStyle::flex_row()
        .fixed_size(MARK_CORE, MARK_CORE)
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::Center)
}

fn mark_inner_layout() -> LayoutStyle {
    LayoutStyle::flex_row().fixed_size(MARK_INNER, MARK_INNER)
}

impl StyleAccess for RadioGroupBuilder {
    fn style_mut(&mut self) -> &mut sigil_style::Style {
        &mut self.style_accumulator
    }
}

impl WidgetLayoutExt for RadioGroupBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl IntoNode for RadioGroupBuilder {
    fn build(self, cx: &mut sigil_kernel::Cx) -> sigil_kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use sigil_kernel::Cx;
    use sigil_reactive::{Runtime, signal};

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
            assert_eq!(node.a11y.role, sigil_kernel::Role::RadioGroup);
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
            assert_eq!(row.a11y.role, sigil_kernel::Role::RadioButton);
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

    #[test]
    fn mark_ring_clears_non_text_contrast() {
        use sigil_style::Rgba8;
        use sigil_style::scale::ScaleRole;
        use sigil_style::theme::Theme;

        // keep luminance arithmetic bit-stable for goldens
        #[allow(clippy::suboptimal_flops)]
        fn luminance(c: Rgba8) -> f64 {
            let ch = |v: u8| {
                let v = f64::from(v) / 255.0;
                if v <= 0.039_28 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * ch(c.r) + 0.7152 * ch(c.g) + 0.0722 * ch(c.b)
        }

        // The `bg-gray-9` ring must clear the WCAG 1.4.11 non-text
        // 3:1 floor against the surface it floats on, in both themes.
        // The previous flat gray-7 mark measured 1.49:1 on light
        // (Fable visual adversarial 2026-07-13).
        for (name, theme) in [("light", Theme::default()), ("dark", Theme::dark())] {
            let ring = theme.scale(ScaleRole::Gray).step(9);
            let (lr, ls) = (luminance(ring), luminance(theme.surface));
            let ratio = (lr.max(ls) + 0.05) / (lr.min(ls) + 0.05);
            assert!(
                ratio >= 3.0,
                "{name}: gray-9 ring on surface measures {ratio:.2}:1, below the 3:1 non-text floor"
            );
        }
    }
}
