const MAX_SPAN_DAYS: f64 = 100.0 * 365.0;
const MAX_DATE_DIGITS: usize = 4;
const UNITS: [(&str, f64); 8] = [
    ("ms", 1.0 / 86_400_000.0),
    ("s", 1.0 / 86_400.0),
    ("m", 1.0 / 1_440.0),
    ("h", 1.0 / 24.0),
    ("d", 1.0),
    ("w", 7.0),
    ("M", 30.0),
    ("y", 365.0),
];

enum Field {
    Duration(f64),
    Date,
    Other,
}

pub(super) fn bounded(source: &str) -> bool {
    if !is_gantt(source) {
        return true;
    }
    let mut span = 0.0;
    for field in source.lines().filter_map(task_fields).flatten() {
        match classify(field) {
            Field::Duration(days) if days.is_finite() => span += days.abs(),
            Field::Duration(_) => return false,
            Field::Date if !date_in_range(field) => return false,
            Field::Date | Field::Other => {}
        }
    }
    span <= MAX_SPAN_DAYS
}

fn is_gantt(source: &str) -> bool {
    source.lines().any(|line| line.trim() == "gantt")
}

fn task_fields(line: &str) -> Option<std::str::Split<'_, char>> {
    let line = line.split("%%").next().unwrap_or_default().trim();
    if line.starts_with("title") || line.starts_with("acc") {
        return None;
    }
    line.split_once(':').map(|(_, fields)| fields.split(','))
}

fn classify(field: &str) -> Field {
    let field = field.trim();
    let field = field.strip_prefix("until ").unwrap_or(field);
    if let Some(days) = duration(field) {
        return Field::Duration(days);
    }
    let dashes = field.chars().filter(|ch| *ch == '-').count();
    let time = field.contains(':') && field.chars().all(|ch| ch.is_ascii_digit() || ch == ':');
    if dashes >= 2 || time {
        Field::Date
    } else {
        Field::Other
    }
}

fn duration(field: &str) -> Option<f64> {
    UNITS.iter().find_map(|(unit, days)| {
        let number = field.strip_suffix(unit)?.parse::<f64>().ok()?;
        Some(number * days)
    })
}

fn date_in_range(field: &str) -> bool {
    field
        .split(|ch: char| !ch.is_ascii_digit())
        .all(|digits| digits.len() <= MAX_DATE_DIGITS)
}
