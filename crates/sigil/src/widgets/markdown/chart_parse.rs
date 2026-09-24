use super::parse::Block;

pub(super) fn fence_block(lang: &str, body: String) -> Block {
    if lang == "mermaid" {
        Block::Mermaid(body)
    } else if matches!(lang, "chart" | "xychart" | "xychart-beta") {
        chart_block(&body, lang.starts_with("xychart"))
    } else {
        Block::Code {
            lang: lang.to_string(),
            body,
        }
    }
}

type ChartData = (Option<String>, Vec<(String, f64)>);

fn chart_block(body: &str, xychart: bool) -> Block {
    chart_rows(body, xychart).map_or_else(
        || Block::Mermaid(body.to_string()),
        |(title, rows)| Block::Chart { title, rows },
    )
}

fn chart_rows(source: &str, xychart: bool) -> Option<ChartData> {
    let lines: Vec<&str> = source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("%%"))
        .collect();
    if xychart
        || lines
            .first()
            .is_some_and(|line| line.starts_with("xychart"))
    {
        return xychart_rows(if xychart { &lines } else { lines.get(1..)? });
    }
    let mut title = None;
    let mut rows = Vec::new();
    for line in lines {
        if rows.is_empty()
            && line
                .split_whitespace()
                .all(|word| word.parse::<f64>().is_err())
            && !line.contains(':')
            && !line.contains(',')
        {
            title = Some(line.trim_matches('"').to_string());
            continue;
        }
        rows.push(pair(line)?);
    }
    (!rows.is_empty()).then_some((title, rows))
}

fn xychart_rows(lines: &[&str]) -> Option<ChartData> {
    let mut title = None;
    let mut labels = Vec::new();
    let mut values = Vec::new();
    for line in lines {
        if let Some(rest) = line.strip_prefix("title") {
            title = Some(rest.trim().trim_matches('"').to_string());
        } else if line.starts_with("x-axis") {
            labels = csv_items(line);
        } else if line.starts_with("bar") || line.starts_with("line") {
            values = csv_items(line)
                .into_iter()
                .filter_map(|item| item.parse::<f64>().ok())
                .collect();
        }
    }
    if values.is_empty() {
        return None;
    }
    let rows = values
        .iter()
        .enumerate()
        .map(|(i, value)| {
            (
                labels
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| format!("{}", i + 1)),
                *value,
            )
        })
        .collect();
    Some((title, rows))
}

fn csv_items(line: &str) -> Vec<String> {
    let Some(open) = line.find('[') else {
        return Vec::new();
    };
    let Some(close) = line.rfind(']') else {
        return Vec::new();
    };
    if close <= open {
        return Vec::new();
    }
    line[open + 1..close]
        .split(',')
        .map(|item| item.trim().trim_matches('"').to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

fn pair(line: &str) -> Option<(String, f64)> {
    if let Some((label, value)) = line.rsplit_once(':').or_else(|| line.rsplit_once(',')) {
        let value: f64 = value.trim().parse().ok()?;
        let label = label.trim().trim_matches('"').to_string();
        return (!label.is_empty() && value >= 0.0).then_some((label, value));
    }
    let value = line.split_whitespace().next_back()?;
    let label = line[..line.len() - value.len()].trim().trim_matches('"');
    let value: f64 = value.parse().ok()?;
    (!label.is_empty() && value >= 0.0).then_some((label.to_string(), value))
}
