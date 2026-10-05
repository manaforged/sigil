use sigil::prelude::*;

pub(crate) type App = fn() -> Box<dyn IntoNode>;

const SOURCES: &[(&str, &str)] = &[
    (
        "crates/sigil/examples/widget_controls.rs",
        include_str!("../widget_controls.rs"),
    ),
    (
        "crates/sigil/examples/widget_fields.rs",
        include_str!("../widget_fields.rs"),
    ),
    (
        "crates/sigil/examples/widget_overlays.rs",
        include_str!("../widget_overlays.rs"),
    ),
    (
        "crates/sigil/examples/widget_chrome.rs",
        include_str!("../widget_chrome.rs"),
    ),
    (
        "crates/sigil/examples/widget_data.rs",
        include_str!("../widget_data.rs"),
    ),
    (
        "crates/sigil/examples/widget_documents.rs",
        include_str!("../widget_documents.rs"),
    ),
    (
        "crates/sigil/examples/widget_indicators.rs",
        include_str!("../widget_indicators.rs"),
    ),
    (
        "crates/sigil/examples/widget_content.rs",
        include_str!("../widget_content.rs"),
    ),
    (
        "crates/sigil/examples/widget_plumbing.rs",
        include_str!("../widget_plumbing.rs"),
    ),
    (
        "crates/sigil/examples/todo-terminal.rs",
        include_str!("../todo-terminal.rs"),
    ),
    (
        "crates/sigil/examples/preferences-window.rs",
        include_str!("../preferences-window.rs"),
    ),
    (
        "crates/sigil/examples/chat-both.rs",
        include_str!("../chat-both.rs"),
    ),
    (
        "crates/sigil/examples/pomodoro-both.rs",
        include_str!("../pomodoro-both.rs"),
    ),
    (
        "crates/sigil/examples/showcase-dashboard/main.rs",
        include_str!("../showcase-dashboard/main.rs"),
    ),
    (
        "crates/sigil/examples/showcase-editor/main.rs",
        include_str!("../showcase-editor/main.rs"),
    ),
    (
        "crates/sigil/examples/showcase-settings.rs",
        include_str!("../showcase-settings.rs"),
    ),
];

const APPS: &[(&str, App)] = &[
    ("todo-terminal", || {
        sigil::kernel::node(crate::todo_terminal::app())
    }),
    ("preferences-window", || {
        sigil::kernel::node(crate::preferences_window::build())
    }),
    ("chat-both", || {
        sigil::kernel::node(crate::chat_both::app_tree())
    }),
    ("pomodoro-both", || {
        sigil::kernel::node(crate::pomodoro_both::app_tree())
    }),
    ("showcase-dashboard", || {
        sigil::kernel::node(crate::showcase_dashboard::app())
    }),
    ("showcase-editor", || {
        sigil::kernel::node(crate::showcase_editor::app())
    }),
    ("showcase-settings", || {
        sigil::kernel::node(crate::showcase_settings::app())
    }),
];

pub(crate) fn source(path: &str) -> Option<&'static str> {
    SOURCES
        .iter()
        .find(|(name, _)| *name == path)
        .map(|(_, text)| *text)
}

pub(crate) fn app(id: &str) -> Option<App> {
    APPS.iter()
        .find(|(name, _)| *name == id)
        .map(|(_, app)| *app)
}
