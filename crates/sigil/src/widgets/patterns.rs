//! Composition patterns: layout shells and discoverability surfaces built
//! on existing widget primitives.
//!
//! Layout shells (Phase E):
//! * [`app_shell`] — top bar / work / status bar / optional modal overlay.
//! * [`field`] — label-above-control vertical pair for form rows.
//! * [`kv_grid`] — key + value rows for header tables / metadata panes.
//!
//! Discoverability (Phase F):
//! * [`tooltip`] — hover/focus-driven hint over any target node. Thin
//!   re-export of [`super::tooltip`], the existing widget.
//! * [`hint_overlay`] — translucent keybinding cheat-sheet.

use sigil_kernel::{Cx, IntoNode, NodeBuilder, NodeId, Role, TextContent, container, text};
use sigil_layout::prelude::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle, Position};
use sigil_reactive::{Signal, derived};
use sigil_style::{DisplayKind, NodeStyleExt, StyleAtom};

use super::shared::label;
use super::text_field::TextFieldBuilder;
use super::tooltip::{TooltipBuilder, tooltip as tooltip_widget};

/// Fixed top-bar height in logical pixels.
const TOP_BAR_HEIGHT: f32 = 48.0;
/// Fixed status-bar height in logical pixels.
const STATUS_BAR_HEIGHT: f32 = 28.0;
/// Inter-element gap between label and control in [`field`].
const FIELD_GAP: f32 = 4.0;
/// Inter-row gap in [`kv_grid`].
const KV_GRID_ROW_GAP: f32 = 4.0;
/// Inter-column gap inside each [`kv_grid`] row.
const KV_GRID_COLUMN_GAP: f32 = 12.0;

/// Four-section application shell: top bar, work area, status bar, and an
/// optional modal overlay. The root is a `flex_column` filling the parent.
///
/// `top` sits at a fixed 48 px, `work` flex-grows to fill the remainder,
/// `status` sits at a fixed 28 px, and `modal` (when `Some`) is appended as
/// an absolutely-positioned sibling so it overlays the column.
///
/// ```ignore
/// use sigil::prelude::*;
///
/// patterns::app_shell(
///     container().child(text("title")),
///     container().child(text("work")),
///     status_bar([chip("READY", ChipTone::Success)]),
///     None::<NodeBuilder>,
/// )
/// ```
#[must_use]
pub fn app_shell(
    top: impl IntoNode + 'static,
    work: impl IntoNode + 'static,
    status: impl IntoNode + 'static,
    modal: Option<impl IntoNode + 'static>,
) -> NodeBuilder {
    let mut root = container()
        .role(Role::GenericContainer)
        .layout(LayoutStyle::flex_column().fill())
        .child(top_section(top))
        .child(work_section(work))
        .child(status_section(status));
    if let Some(overlay) = modal {
        root = root.child(modal_overlay(overlay));
    }
    root
}

fn top_section(child: impl IntoNode + 'static) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .layout(LayoutStyle::flex_row().fill_width().height(TOP_BAR_HEIGHT))
        .child(child)
}

fn work_section(child: impl IntoNode + 'static) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .layout(LayoutStyle::flex_column().fill_width().grow(1.0))
        .child(child)
}

fn status_section(child: impl IntoNode + 'static) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .height(STATUS_BAR_HEIGHT),
        )
        .child(child)
}

fn modal_overlay(child: impl IntoNode + 'static) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .layout(
            LayoutStyle::flex_column()
                .position(Position::Absolute)
                .fill(),
        )
        .child(child)
}

/// Label-above-control form row: a `flex_column` containing the label
/// followed by the supplied control with a 4 px gap between them.
///
/// ```ignore
/// use sigil::prelude::*;
///
/// patterns::field("Username", text_field().placeholder("you@host"))
/// ```
#[must_use]
pub fn field(label_text: impl Into<TextContent>, control: impl IntoNode + 'static) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .layout(LayoutStyle::flex_column().fill_width().gap(FIELD_GAP))
        .child(label(label_text))
        .child(control)
}

/// Variant of [`field`] with a reactive validation-error slot.
///
/// `error` is a `Signal<Option<String>>` — when it reads `Some("…")`
/// a red helper-text node appears under the control with the
/// supplied message; when `None`, the slot collapses via
/// `Display::None` and contributes no layout box.
///
/// ```ignore
/// use sigil::prelude::*;
/// use sigil_reactive::{signal, derived};
///
/// let email = signal(String::new());
/// let email_error = derived(move || {
///     let v = email.get();
///     (!v.contains('@')).then(|| String::from("Email must contain @"))
/// });
///
/// patterns::validated_field(
///     "Email",
///     text_field(email).placeholder("you@example.com"),
///     email_error,
/// )
/// ```
#[must_use]
pub fn validated_field(
    label_text: impl Into<TextContent>,
    control: TextFieldBuilder,
    error: Signal<Option<String>>,
) -> NodeBuilder {
    let message: Signal<String> = derived(move || error.get().unwrap_or_default());
    let visible: Signal<bool> = derived(move || error.get().is_some());
    // Wire the error state into the control's invalid border so the
    // field itself enters an error state, not just the helper text.
    // Without this the reviewer (2026-05-21) flagged validated_field as
    // "carrying the error in the helper text only" — a semantic-drift
    // a11y failure (the field doesn't signal invalidity to assistive
    // tech or sighted users glancing at the control).
    let invalid_signal: Signal<bool> = derived(move || error.get().is_some());
    let control = control.invalid_when(invalid_signal);
    container()
        .role(Role::GenericContainer)
        .layout(LayoutStyle::flex_column().fill_width().gap(FIELD_GAP))
        .child(label(label_text))
        .child(control)
        .child(
            // step-12 instead of step-11: at the 12 px `text-xs` size
            // antialiasing dilutes effective contrast by ~1.5 stops vs.
            // the pure colour pair, pushing step-11 ruby (~5.4:1 solid)
            // into the ~3.5:1 measured zone — below the 4.5:1 floor for
            // small body text. Step 12 ruby (#641723) measures ~12:1
            // against light surfaces and survives the AA penalty.
            text(message)
                .role(Role::Label)
                .class("text-error-12 text-xs font-medium")
                .style_reactive(move || {
                    if visible.get() {
                        ERROR_VISIBLE
                    } else {
                        ERROR_HIDDEN
                    }
                }),
        )
}

static ERROR_VISIBLE: &[StyleAtom] = &[];
static ERROR_HIDDEN: &[StyleAtom] = &[StyleAtom::Display(DisplayKind::None)];

/// Two-column key/value grid: each row is a `flex_row` with the key on the
/// left and the value (rendered in the monospace class) on the right.
///
/// ```ignore
/// use sigil::prelude::*;
///
/// patterns::kv_grid(&[
///     ("Content-Type", "application/json"),
///     ("Status", "200 OK"),
/// ])
/// ```
#[must_use]
pub fn kv_grid(rows: &[(&'static str, &'static str)]) -> NodeBuilder {
    let mut root = container()
        .role(Role::GenericContainer)
        .layout(LayoutStyle::flex_column().fill_width().gap(KV_GRID_ROW_GAP));
    for (key, value) in rows {
        root = root.child(kv_grid_row(key, value));
    }
    root
}

fn kv_grid_row(key: &'static str, value: &'static str) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .layout(LayoutStyle::flex_row().fill_width().gap(KV_GRID_COLUMN_GAP))
        .child(label(key))
        .child(text(value).class("font-mono"))
}

/// Wrap `target` with a hover/focus tooltip that shows `hint`.
///
/// Direct composition helper — equivalent to
/// `tooltip(hint).child(target)`. Provided as a free function in the
/// `patterns::` namespace so authoring code reads top-down:
///
/// ```no_run
/// use sigil::prelude::*;
/// use sigil::widgets::patterns;
///
/// # fn ui(_cx: &mut Cx) {
/// let _node = patterns::tooltip(button("Save"), "Save the file");
/// # }
/// ```
///
/// The wrapped target inherits the existing tooltip widget's
/// `pointer_enter` / `pointer_leave` driven visibility and ARIA
/// description wiring.
#[must_use]
pub fn tooltip(target: impl IntoNode + 'static, hint: impl Into<String>) -> TooltipBuilder {
    tooltip_widget(hint).child(target)
}

/// A single chord/description pair displayed by [`hint_overlay`].
///
/// The borrowed form keeps the call-site terse — `&[("Ctrl+S",
/// "Save"), ("Ctrl+Z", "Undo")]` — without forcing `String`
/// allocations for static keybinding tables.
pub type HintEntry<'a> = (&'a str, &'a str);

/// Builder returned by [`hint_overlay`].
///
/// Renders a translucent column of `chord — description` rows. Always
/// visible while mounted; hosts that want toggleable visibility wrap
/// the builder in a reactive `if let` against a `Signal<bool>` at the
/// call site (the typical "press `?` to toggle the cheat sheet" UX).
pub struct HintOverlayBuilder {
    entries: Vec<(String, String)>,
}

impl HintOverlayBuilder {
    /// Construct a hint overlay seeded with the supplied entries.
    #[must_use]
    pub fn new(hints: &[HintEntry<'_>]) -> Self {
        let entries = hints
            .iter()
            .map(|(chord, desc)| ((*chord).to_owned(), (*desc).to_owned()))
            .collect();
        Self { entries }
    }

    /// Push an additional hint onto the overlay.
    #[must_use]
    pub fn entry(mut self, chord: impl Into<String>, description: impl Into<String>) -> Self {
        self.entries.push((chord.into(), description.into()));
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let mut root = container()
            .role(Role::GenericContainer)
            .label("Keyboard shortcuts")
            .layout(
                LayoutStyle::flex_column()
                    .padding(12.0)
                    .gap(6.0)
                    .align_items(AlignItems::Start)
                    .justify_content(JustifyContent::Start),
            )
            .class("rounded-md")
            .class(OVERLAY_BASE);

        for (chord, desc) in self.entries {
            root = root.child(hint_row(&chord, &desc));
        }
        root
    }
}

impl IntoNode for HintOverlayBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.into_node_builder().build(cx)
    }
}

static OVERLAY_BASE: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("surface"),
    StyleAtom::TextThemeRef("on-surface"),
];

fn hint_row(chord: &str, description: &str) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .layout(
            LayoutStyle::flex_row()
                .gap(8.0)
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::Start),
        )
        .child(text(chord.to_owned()).class("font-mono"))
        .child(text(description.to_owned()))
}

/// Construct a hint overlay from a slice of `(chord, description)` pairs.
///
/// Intended for "press `?` for shortcuts" UX. The overlay is always
/// visible while mounted; gate visibility from the host using a
/// reactive `if`:
///
/// ```no_run
/// use sigil::prelude::*;
/// use sigil::widgets::patterns;
///
/// # fn ui(_cx: &mut Cx) {
/// let visible = signal(false);
/// // Mount/unmount the overlay reactively based on `visible.get()`.
/// let _hints = patterns::hint_overlay(&[
///     ("Ctrl+S", "Save"),
///     ("Ctrl+Z", "Undo"),
///     ("?",      "Toggle this overlay"),
/// ]);
/// # let _ = visible;
/// # }
/// ```
#[must_use]
pub fn hint_overlay(hints: &[HintEntry<'_>]) -> HintOverlayBuilder {
    HintOverlayBuilder::new(hints)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use sigil_kernel::container as kernel_container;
    use sigil_reactive::Runtime;

    #[test]
    fn app_shell_without_modal_has_three_children() {
        Runtime::scope("app-shell-no-modal", || {
            let mut cx = Cx::new_root();
            let id = app_shell(container(), container(), container(), None::<NodeBuilder>)
                .build(&mut cx);
            let node = cx.get(id).expect("app-shell root exists");
            assert_eq!(node.a11y.role, Role::GenericContainer);
            assert_eq!(
                node.children().len(),
                3,
                "without modal, app_shell is top + work + status"
            );
        });
    }

    #[test]
    fn app_shell_with_modal_has_four_children() {
        Runtime::scope("app-shell-with-modal", || {
            let mut cx = Cx::new_root();
            let id =
                app_shell(container(), container(), container(), Some(container())).build(&mut cx);
            let node = cx.get(id).expect("app-shell root exists");
            assert_eq!(
                node.children().len(),
                4,
                "with modal, app_shell appends the overlay as a 4th sibling"
            );
        });
    }

    #[test]
    fn field_holds_label_and_control_in_order() {
        Runtime::scope("field-static-label", || {
            let mut cx = Cx::new_root();
            let id = field("Username", container()).build(&mut cx);
            let node = cx.get(id).expect("field root exists");
            assert_eq!(node.a11y.role, Role::GenericContainer);
            assert_eq!(
                node.children().len(),
                2,
                "field produces exactly label + control"
            );
        });
    }

    #[test]
    fn validated_field_emits_three_children() {
        use crate::widgets::text_field::text_field;
        Runtime::scope("validated-field-shape", || {
            let mut cx = Cx::new_root();
            let error: sigil_reactive::Signal<Option<String>> = sigil_reactive::signal(None);
            let value = sigil_reactive::signal(String::new());
            let id = validated_field("Email", text_field(value), error).build(&mut cx);
            let node = cx.get(id).expect("validated field root");
            assert_eq!(
                node.children().len(),
                3,
                "validated_field is label + control + error-slot",
            );
        });
    }

    #[test]
    fn kv_grid_emits_one_row_per_input() {
        Runtime::scope("kv-grid-three-rows", || {
            let mut cx = Cx::new_root();
            let id = kv_grid(&[
                ("Content-Type", "application/json"),
                ("Status", "200 OK"),
                ("Length", "1024"),
            ])
            .build(&mut cx);
            let node = cx.get(id).expect("kv-grid root exists");
            assert_eq!(
                node.children().len(),
                3,
                "kv_grid produces one row child per input tuple"
            );
        });
    }

    #[test]
    fn kv_grid_empty_input_has_no_rows() {
        Runtime::scope("kv-grid-empty", || {
            let mut cx = Cx::new_root();
            let id = kv_grid(&[]).build(&mut cx);
            let node = cx.get(id).expect("kv-grid root exists");
            assert!(
                node.children().is_empty(),
                "empty input must produce zero rows"
            );
        });
    }

    #[test]
    fn patterns_tooltip_wraps_target() {
        Runtime::scope("patterns-tooltip", || {
            let _builder = tooltip(kernel_container(), "Save the file");
        });
    }

    #[test]
    fn hint_overlay_records_entries() {
        let overlay = hint_overlay(&[("Ctrl+S", "Save"), ("?", "Help")]).entry("Esc", "Close");
        assert_eq!(overlay.entries.len(), 3);
        assert_eq!(overlay.entries[0].0, "Ctrl+S");
        assert_eq!(overlay.entries[2].1, "Close");
    }

    #[test]
    fn hint_overlay_builds() {
        Runtime::scope("hint-overlay-build", || {
            let _node = hint_overlay(&[("Ctrl+S", "Save")]).into_node_builder();
        });
    }
}
