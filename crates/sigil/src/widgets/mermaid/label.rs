pub(super) const LABEL_COLS: usize = 32;

pub(super) fn breaks(raw: &str) -> Vec<String> {
    raw.replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("<br>", "\n")
        .lines()
        .map(|line| line.trim().to_string())
        .collect()
}

pub(super) fn flat(raw: &str) -> String {
    breaks(raw).join(" ")
}

pub(super) fn wrapped(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in breaks(raw) {
        let mut row = String::new();
        for word in line.split_whitespace() {
            let need = row.chars().count() + usize::from(!row.is_empty()) + word.chars().count();
            if !row.is_empty() && need > LABEL_COLS {
                out.push(std::mem::take(&mut row));
            }
            if !row.is_empty() {
                row.push(' ');
            }
            row.push_str(word);
        }
        out.push(row);
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

pub(super) fn width(lines: &[String]) -> usize {
    lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0)
}
