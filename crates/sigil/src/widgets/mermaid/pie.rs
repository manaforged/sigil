const BAR: usize = 24;

pub(super) fn draw(source: &str) -> Option<Vec<String>> {
    let mut lines = source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("%%"));
    let head = lines.next()?;
    if head != "pie" && !head.starts_with("pie ") {
        return None;
    }
    let mut title = head
        .split_once("title")
        .map(|(_, rest)| rest.trim().to_string())
        .filter(|text| !text.is_empty());
    let mut slices = Vec::new();
    for line in lines {
        if let Some(rest) = line.strip_prefix("title") {
            title = Some(rest.trim().to_string());
            continue;
        }
        if let Some(slice) = slice(line) {
            slices.push(slice);
        }
    }
    if slices.is_empty() {
        return None;
    }
    Some(render(title, &slices))
}

fn slice(line: &str) -> Option<(String, f64)> {
    let (label, value) = line.rsplit_once(':')?;
    let value: f64 = value.trim().parse().ok()?;
    let label = label.trim().trim_matches('"').to_string();
    (!label.is_empty() && value >= 0.0).then_some((label, value))
}

fn render(title: Option<String>, slices: &[(String, f64)]) -> Vec<String> {
    let total: f64 = slices.iter().map(|(_, value)| value).sum();
    let width = slices
        .iter()
        .map(|(label, _)| label.chars().count())
        .max()
        .unwrap_or(0);
    let mut out = Vec::new();
    if let Some(title) = title {
        out.push(title);
    }
    for (label, value) in slices {
        let share = if total > 0.0 { value / total } else { 0.0 };
        let filled = cells(share);
        let pad = " ".repeat(width.saturating_sub(label.chars().count()));
        out.push(format!(
            "{label}{pad}  {}{}  {:>5.1}%",
            "█".repeat(filled),
            "░".repeat(BAR.saturating_sub(filled)),
            share * 100.0
        ));
    }
    out
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "the share is clamped to the unit interval before the cast"
)]
fn cells(share: f64) -> usize {
    ((share.clamp(0.0, 1.0) * BAR as f64).round() as usize).min(BAR)
}
