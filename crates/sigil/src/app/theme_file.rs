use crate::style::{DateOrder, Direction, Locale, Rgba8, StringKey, Theme, ThemeValue};

pub(super) fn load(path: &std::path::Path) -> Option<Theme> {
    let input = std::fs::read_to_string(path).ok()?;
    parse(&input)
}

pub fn parse(input: &str) -> Option<Theme> {
    let doc = input.parse::<toml::Table>().ok()?;
    let mut theme = match doc.get("base").and_then(toml::Value::as_str) {
        Some("dark") => Theme::dark(),
        Some("console_dark") => Theme::console_dark(),
        Some("light") | None => Theme::light(),
        Some(_) => return None,
    };
    let tokens = doc.get("tokens")?.as_table()?;
    for (name, value) in tokens {
        let hex = value.as_str()?;
        let color = parse_hex(hex)?;
        theme.register_token(name, ThemeValue::Color(color));
    }
    if let Some(value) = doc.get("direction") {
        theme.direction = Direction::from_name(value.as_str()?)?;
    }
    theme.locale = parse_locale(&doc, theme.locale)?;
    if let Some(value) = doc.get("strings") {
        for (name, text) in value.as_table()? {
            let key = StringKey::from_name(name)?;
            theme.strings.set(key, text.as_str()?.to_owned());
        }
    }
    Some(theme)
}

fn parse_locale(doc: &toml::Table, current: Locale) -> Option<Locale> {
    let table = match doc.get("locale") {
        None => return Some(current),
        Some(toml::Value::String(name)) => return Locale::from_name(name),
        Some(toml::Value::Table(table)) => table,
        Some(_) => return None,
    };
    let mut locale = match table.get("preset") {
        Some(name) => Locale::from_name(name.as_str()?)?,
        None => current,
    };
    for (name, value) in table {
        apply_locale_field(&mut locale, name, value)?;
    }
    Some(locale)
}

fn apply_locale_field(locale: &mut Locale, name: &str, value: &toml::Value) -> Option<()> {
    match name {
        "preset" => {}
        "decimal" => locale.decimal = single_char(value)?,
        "group" => {
            let text = value.as_str()?;
            locale.group = if text.is_empty() {
                None
            } else {
                Some(single_char(value)?)
            };
        }
        "group_size" => {
            locale.group_size = u8::try_from(value.as_integer()?)
                .ok()
                .filter(|size| *size > 0)?;
        }
        "zero" => locale.zero = single_char(value)?,
        "date_order" => locale.date_order = DateOrder::from_name(value.as_str()?)?,
        "date_separator" => locale.date_separator = single_char(value)?,
        _ => return None,
    }
    Some(())
}

fn single_char(value: &toml::Value) -> Option<char> {
    let mut chars = value.as_str()?.chars();
    let first = chars.next()?;
    chars.next().is_none().then_some(first)
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

pub(crate) fn launch(path: Option<&std::path::Path>) -> Option<crate::app::ThemeWatcher> {
    let path = path?;
    if let Some(theme) = load(path) {
        crate::app::set_theme(theme);
    } else {
        tracing::warn!("sigil-app: cannot load theme file {}", path.display());
    }
    crate::app::watch_theme(path)
        .inspect_err(|error| {
            tracing::warn!(
                "sigil-app: cannot watch theme file {}: {error}",
                path.display()
            );
        })
        .ok()
}

#[cfg(test)]
#[path = "theme_file_tests.rs"]
mod tests;
