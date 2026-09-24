use sigil::prelude::*;

use super::spec::Build;
use super::{chrome, content, controls, data, inputs, overlays, plumbing};

pub(crate) type App = fn(&mut Cx) -> NodeId;

const BUILDERS: [&[(&str, Build)]; 7] = [
    plumbing::BUILDERS,
    controls::BUILDERS,
    inputs::BUILDERS,
    data::BUILDERS,
    content::BUILDERS,
    chrome::BUILDERS,
    overlays::BUILDERS,
];

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
        "crates/sigil/examples/showcase-dashboard.rs",
        include_str!("../showcase-dashboard.rs"),
    ),
    (
        "crates/sigil/examples/showcase-editor.rs",
        include_str!("../showcase-editor.rs"),
    ),
    (
        "crates/sigil/examples/showcase-settings.rs",
        include_str!("../showcase-settings.rs"),
    ),
];

const APPS: &[(&str, App)] = &[
    ("todo-terminal", crate::todo_terminal::app),
    ("preferences-window", crate::preferences_window::build),
    ("chat-both", crate::chat_both::app_tree),
    ("pomodoro-both", crate::pomodoro_both::app_tree),
    ("showcase-dashboard", crate::showcase_dashboard::app),
    ("showcase-editor", crate::showcase_editor::app),
    ("showcase-settings", crate::showcase_settings::app),
];

pub(crate) fn builder(id: &str) -> Option<Build> {
    BUILDERS
        .iter()
        .flat_map(|table| table.iter())
        .find(|(name, _)| *name == id)
        .map(|(_, build)| *build)
}

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
