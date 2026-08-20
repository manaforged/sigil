//! Terminal host for Sigil.
//!
//! Same [`Host`], [`TreeRoot`], and widget builders as
//! desktop. This crate maps keys to [`sigil_hal::ShellEvent`] and paints
//! [`sigil_app::Painted`] onto a cell grid. It is not a second widget kit.

#![deny(missing_docs)]

mod keys;
mod paint;
mod run;

pub use keys::{key_down, map_key};
pub use paint::{Grid, raster};
pub use run::{present, run, step};
pub use sigil_app::{AppConfig, AppResult, Error, Host, Painted, TreeRoot};
pub use sigil_events::EventRouter;

/// Widget builders — the same catalog desktop apps import from the facade.
pub use sigil_widgets as widgets;

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, missing_docs)]
mod tests {
    use sigil_hal::Key;
    use sigil_kernel::{Cx, NodeId, container, text};
    use sigil_reactive::{derived, signal};
    use sigil_widgets::button;

    use super::{key_down, present};

    fn counter(cx: &mut Cx) -> NodeId {
        let count = signal(0_i32);
        let label = derived(move || format!("Count: {}", count.get()));
        cx.add(
            container()
                .child(text(label))
                .child(button("Increment").activate(move || {
                    count.set(count.peek() + 1);
                })),
        )
    }

    #[test]
    fn same_tree_paints_label() {
        let grid = present(counter, 40, 12, &[]);
        assert!(grid.contains("Count: 0"), "expected Count: 0 in:\n{grid}");
        assert!(grid.contains("Increment"), "expected Increment in:\n{grid}");
    }

    #[test]
    fn enter_increments_the_same_button() {
        let grid = present(counter, 40, 12, &[key_down(Key::Enter)]);
        assert!(
            grid.contains("Count: 1"),
            "expected Count: 1 after Enter in:\n{grid}"
        );
    }

    #[test]
    fn progress_paints_a_block_bar() {
        use sigil_reactive::signal;
        use sigil_widgets::progress;
        let grid = present(
            |cx: &mut Cx| {
                let value = signal(0.6_f32);
                cx.add(progress(value, 1.0))
            },
            40,
            8,
            &[],
        );
        assert!(grid.contains('█'), "expected a filled bar in:\n{grid}");
    }

    #[test]
    fn chart_bars_use_the_same_block_fill() {
        use sigil_widgets::chart;
        let grid = present(
            |cx: &mut Cx| cx.add(chart([1.0, 3.0, 2.0]).bar()),
            40,
            16,
            &[],
        );
        assert!(grid.contains('█'), "expected chart bars in:\n{grid}");
    }

    #[test]
    fn mermaid_fence_paints_boxes() {
        use sigil_reactive::signal;
        use sigil_widgets::markdown;
        let grid = present(
            |cx: &mut Cx| {
                let src = signal("```mermaid\ngraph TD\nA[Go] --> B[Stop]\n```".to_string());
                cx.add(markdown(src))
            },
            40,
            16,
            &[],
        );
        assert!(grid.contains("Go"), "expected Go in:\n{grid}");
        assert!(grid.contains("Stop"), "expected Stop in:\n{grid}");
        assert!(grid.contains('┌'), "expected a box in:\n{grid}");
    }

    #[test]
    fn map_enter_is_the_shell_enter_key() {
        let event = crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
        );
        let mapped = super::map_key(&event).expect("enter maps");
        assert_eq!(mapped, key_down(Key::Enter));
    }
}
