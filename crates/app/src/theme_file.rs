//! Watched theme files (5/5 Wave 4, T14) — the edit-see loop for
//! style iteration: point `AppConfig::theme_file` at a TOML file and
//! edits repaint live, no rebuild.
//!
//! Format:
//!
//! ```toml
//! base = "dark"          # optional: "light" (default), "dark", "console_dark"
//!
//! [tokens]
//! accent = "#7c3aed"
//! surface-hover = "#f5f5f5"
//! ```

use std::path::PathBuf;

use sigil_style::{Rgba8, Theme, ThemeValue};

/// Load + parse a theme file. Missing file, unreadable content, or a
/// parse error returns `None` — the caller falls back to the base theme
/// and logs (an edit-see loop must never kill the app for a bad save).
pub(super) fn load(path: &std::path::Path) -> Option<Theme> {
    let input = std::fs::read_to_string(path).ok()?;
    parse(&input)
}

fn parse(input: &str) -> Option<Theme> {
    let doc = input.parse::<toml::Table>().ok()?;
    let mut theme = match doc.get("base").and_then(toml::Value::as_str) {
        Some("dark") => Theme::dark(),
        Some("console_dark") => Theme::console_dark(),
        Some("light") | None => Theme::default(),
        Some(_) => return None,
    };
    let tokens = doc.get("tokens")?.as_table()?;
    for (name, value) in tokens {
        let hex = value.as_str()?;
        let color = parse_hex(hex)?;
        theme.register_token(name.clone(), ThemeValue::Color(color));
    }
    Some(theme)
}

/// `#rrggbb` / `#rrggbbaa` → Rgba8. Accepts a leading `0x` too.
// The shifts below are provably in-range (24-bit/32-bit fields), so the
// u8 truncation is intentional, not a bug.
#[allow(clippy::cast_possible_truncation)]
fn parse_hex(hex: &str) -> Option<Rgba8> {
    let digits = hex
        .strip_prefix('#')
        .or_else(|| hex.strip_prefix("0x"))
        .unwrap_or(hex);
    let value = u32::from_str_radix(digits, 16).ok()?;
    match digits.len() {
        6 => Some(Rgba8 {
            r: (value >> 16) as u8,
            g: (value >> 8) as u8,
            b: value as u8,
            a: 255,
        }),
        8 => Some(Rgba8 {
            r: (value >> 24) as u8,
            g: (value >> 16) as u8,
            b: (value >> 8) as u8,
            a: value as u8,
        }),
        _ => None,
    }
}

/// Watch `path` for modifications; each event sends one `()` on the
/// returned channel (coalesced by the caller's frame drain). The watcher
/// thread exits when the channel's receiver drops.
pub(super) fn watch(path: &std::path::Path) -> Option<crossbeam_channel::Receiver<()>> {
    let (tx, rx) = crossbeam_channel::unbounded();
    let parent = path.parent()?.to_path_buf();
    let file = path.to_path_buf();
    let mut watcher =
        notify::recommended_watcher(move |result: Result<notify::Event, notify::Error>| {
            if let Ok(event) = result
                && matches!(
                    event.kind,
                    notify::EventKind::Modify(_) | notify::EventKind::Create(_)
                )
                && event.paths.iter().any(|p| p == &file)
            {
                let _ = tx.send(());
            }
        })
        .ok()?;
    notify::Watcher::watch(&mut watcher, &parent, notify::RecursiveMode::NonRecursive).ok()?;
    // Keep the watcher alive for the process — dropping it stops events.
    // The file is process-scoped state anyway (dev loop).
    let _leaked: &'static _ = Box::leak(Box::new(watcher));
    Some(rx)
}

/// A live theme-file watch: the path plus the wake channel the frame
/// loop drains.
pub(super) struct ThemeWatch {
    pub(super) path: PathBuf,
    pub(super) rx: crossbeam_channel::Receiver<()>,
}

/// Load the watched file and attach the parsed theme to `root`.
/// No-ops on parse failure (a bad save must never kill the loop).
pub(super) fn attach_current(
    cx: &mut sigil_kernel::Cx,
    root: sigil_kernel::NodeId,
    watch: &ThemeWatch,
) {
    if let Some(theme) = load(&watch.path)
        && let Some(node) = cx.get_mut(root)
    {
        node.attach(theme);
    }
}

/// Drain pending watch events, re-attaching on the latest state.
/// Returns true when the theme actually changed this frame.
pub(super) fn drain(
    cx: &mut sigil_kernel::Cx,
    root: sigil_kernel::NodeId,
    watch: &ThemeWatch,
) -> bool {
    let mut changed = false;
    while watch.rx.try_recv().is_ok() {
        attach_current(cx, root, watch);
        changed = true;
    }
    changed
}

/// Resolve the configured theme file path into a running watch.
pub(super) fn start(configured: Option<&std::path::Path>) -> Option<ThemeWatch> {
    let path = configured?;
    let rx = watch(path)?;
    Some(ThemeWatch {
        path: path.to_path_buf(),
        rx,
    })
}

#[cfg(test)]
#[path = "theme_file_tests.rs"]
mod tests;
