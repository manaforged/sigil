use std::path::PathBuf;

use crate::style::{Rgba8, Theme, ThemeValue};

pub(super) fn load(path: &std::path::Path) -> Option<Theme> {
    let input = std::fs::read_to_string(path).ok()?;
    parse(&input)
}

pub fn parse(input: &str) -> Option<Theme> {
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

#[allow(
    clippy::cast_possible_truncation,
    reason = "values are bounded to the target range before the cast"
)]
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

pub(super) fn watcher(
    path: &std::path::Path,
) -> Result<(notify::RecommendedWatcher, crossbeam_channel::Receiver<()>), notify::Error> {
    let (tx, rx) = crossbeam_channel::unbounded();
    let file = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let parent = file
        .parent()
        .map(std::path::Path::to_path_buf)
        .ok_or_else(notify::Error::path_not_found)?;
    let mut watcher =
        notify::recommended_watcher(move |result: Result<notify::Event, notify::Error>| {
            if let Ok(event) = result
                && matches!(
                    event.kind,
                    notify::EventKind::Modify(_) | notify::EventKind::Create(_)
                )
                && event.paths.iter().any(|p| {
                    p == &file || std::fs::canonicalize(p).ok().as_deref() == Some(file.as_path())
                })
            {
                let _ = tx.send(());
            }
        })?;
    notify::Watcher::watch(&mut watcher, &parent, notify::RecursiveMode::NonRecursive)?;
    Ok((watcher, rx))
}

pub(super) struct ThemeWatch {
    pub(super) path: PathBuf,
    rx: Option<crossbeam_channel::Receiver<()>>,
    _watcher: Option<notify::RecommendedWatcher>,
}

pub(super) fn attach_current(
    cx: &mut crate::kernel::Cx,
    root: crate::kernel::NodeId,
    watch: &ThemeWatch,
) {
    if let Some(theme) = load(&watch.path)
        && let Some(node) = cx.get_mut(root)
    {
        node.attach(theme);
    }
}

pub(super) fn drain(
    cx: &mut crate::kernel::Cx,
    root: crate::kernel::NodeId,
    watch: &ThemeWatch,
) -> bool {
    let Some(rx) = &watch.rx else {
        return false;
    };
    let mut changed = false;
    while rx.try_recv().is_ok() {
        changed = true;
    }
    if changed {
        attach_current(cx, root, watch);
    }
    changed
}

pub(super) fn start(configured: Option<&std::path::Path>) -> Option<ThemeWatch> {
    let path = configured?;
    let (watcher, rx) = match watcher(path) {
        Ok((watcher, rx)) => (Some(watcher), Some(rx)),
        Err(error) => {
            tracing::warn!(
                "sigil-app: cannot watch theme file {}: {error}",
                path.display()
            );
            (None, None)
        }
    };
    Some(ThemeWatch {
        path: path.to_path_buf(),
        rx,
        _watcher: watcher,
    })
}

#[cfg(test)]
#[path = "theme_file_tests.rs"]
mod tests;
