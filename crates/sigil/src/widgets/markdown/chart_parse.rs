use super::parse::Block;

pub(super) fn fence_block(lang: &str, body: String) -> Block {
    if lang == "mermaid" {
        let xychart = body
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty() && !line.starts_with("%%"))
            .is_some_and(|line| line.starts_with("xychart"));
        if xychart {
            chart_block(&body, false)
        } else {
            Block::Mermaid(body)
        }
    } else if matches!(lang, "chart" | "xychart" | "xychart-beta") {
        chart_block(&body, lang.starts_with("xychart"))
    } else {
        Block::Code {
            lang: lang.to_string(),
            body,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ChartData {
    pub(super) title: Option<String>,
    pub(super) labels: Vec<String>,
    pub(super) values: Vec<f32>,
    pub(super) series: Vec<Vec<f32>>,
    pub(super) line: bool,
}

fn chart_block(body: &str, xychart: bool) -> Block {
    chart_rows(body, xychart).map_or_else(|| Block::Mermaid(body.to_string()), Block::Chart)
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
    if rows.is_empty() {
        return None;
    }
    let (labels, values) = rows.into_iter().unzip();
    Some(ChartData {
        title,
        labels,
        values,
        series: Vec::new(),
        line: false,
    })
}

fn xychart_rows(lines: &[&str]) -> Option<ChartData> {
    let mut title = None;
    let mut labels = Vec::new();
    let mut series = Vec::new();
    let mut mark = None;
    for line in lines {
        if let Some(rest) = line.strip_prefix("title") {
            title = Some(rest.trim().trim_matches('"').to_string());
        } else if line.starts_with("x-axis") {
            labels = csv_items(line);
        } else if line.starts_with("bar") || line.starts_with("line") {
            let is_line = line.starts_with("line");
            if mark.is_some_and(|held| held != is_line) {
                return None;
            }
            mark = Some(is_line);
            let values = csv_items(line)
                .into_iter()
                .map(|item| item.parse::<f32>().ok().filter(|value| value.is_finite()))
                .collect::<Option<Vec<_>>>()?;
            if values.is_empty() {
                return None;
            }
            series.push(values);
        }
    }
    let count = series.iter().map(Vec::len).max()?;
    let labels = (0..count)
        .map(|index| {
            labels
                .get(index)
                .cloned()
                .unwrap_or_else(|| (index + 1).to_string())
        })
        .collect();
    let mut series = series.into_iter();
    Some(ChartData {
        title,
        labels,
        values: series.next()?,
        series: series.collect(),
        line: mark?,
    })
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
        .collect()
}

fn pair(line: &str) -> Option<(String, f32)> {
    let (label, value) = if let Some(pair) = line.rsplit_once(':').or_else(|| line.rsplit_once(','))
    {
        pair
    } else {
        let value = line.split_whitespace().next_back()?;
        (&line[..line.len() - value.len()], value)
    };
    let label = label.trim().trim_matches('"').to_string();
    let value: f32 = value.trim().parse().ok()?;
    (!label.is_empty() && value.is_finite() && value >= 0.0).then_some((label, value))
}
