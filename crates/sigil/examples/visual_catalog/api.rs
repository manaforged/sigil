use std::path::Path;

use serde_json::{Value, json};

const SOURCE_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src");

pub(crate) struct ApiRow {
    pub(crate) name: String,
    pub(crate) signature: String,
    pub(crate) purpose: String,
}

impl ApiRow {
    fn json(&self) -> Value {
        json!({"name": self.name, "signature": self.signature, "purpose": self.purpose})
    }
}

pub(crate) struct ApiMap {
    rows: Vec<ApiRow>,
    aliases: Vec<(String, String)>,
}

impl ApiMap {
    pub(crate) fn load() -> Self {
        let mut map = Self {
            rows: Vec::new(),
            aliases: Vec::new(),
        };
        map.collect(Path::new(SOURCE_ROOT));
        map
    }

    pub(crate) fn row(&self, name: &str) -> Option<&ApiRow> {
        let mut named = self.rows.iter().filter(|row| row.name == name);
        let Some(first) = named.next() else {
            let (_, source) = self.aliases.iter().find(|(alias, _)| alias == name)?;
            return self.rows.iter().find(|row| &row.name == source);
        };
        if first.purpose.is_empty() {
            return named.find(|row| !row.purpose.is_empty()).or(Some(first));
        }
        Some(first)
    }

    pub(crate) fn rows_for(&self, builder: &str) -> Vec<Value> {
        let Some(head) = self.row(builder) else {
            return Vec::new();
        };
        let mut out = vec![head.json()];
        let kind = builder_type(&head.signature);
        if let Some(row) = kind.and_then(|kind| self.row(kind)) {
            out.push(row.json());
        }
        out
    }

    fn collect(&mut self, dir: &Path) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut paths: Vec<_> = entries
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .collect();
        paths.sort();
        for path in paths {
            if path.is_dir() {
                self.collect(&path);
            } else if path.extension().is_some_and(|ext| ext == "rs")
                && let Ok(text) = std::fs::read_to_string(&path)
            {
                self.parse(&text);
            }
        }
    }

    fn parse(&mut self, text: &str) {
        let lines: Vec<&str> = text.lines().collect();
        for (at, line) in lines.iter().enumerate() {
            if let Some(list) = line.strip_prefix("pub use ") {
                self.aliases.extend(aliases(list));
            } else if let Some(name) = item_name(line) {
                self.rows.push(item_row(name, &lines, at));
            }
        }
    }
}

fn item_row(name: &str, lines: &[&str], at: usize) -> ApiRow {
    let docs: Vec<&str> = lines
        .get(..at)
        .unwrap_or_default()
        .iter()
        .rev()
        .take_while(|l| l.starts_with("///") || l.starts_with("#["))
        .filter_map(|l| l.strip_prefix("///"))
        .map(str::trim)
        .collect();
    let purpose = docs.into_iter().rev().collect::<Vec<_>>().join(" ");
    let tail = lines.get(at..).unwrap_or_default();
    let end = tail
        .iter()
        .position(|l| l.contains(['{', ';']))
        .map_or(tail.len(), |end| end + 1);
    let joined = tail
        .get(..end)
        .unwrap_or_default()
        .iter()
        .map(|l| l.trim())
        .collect::<Vec<_>>()
        .join(" ");
    let signature = joined.split(['{', ';']).next().unwrap_or_default();
    ApiRow {
        name: name.to_owned(),
        signature: signature.trim_start_matches("pub ").trim().to_owned(),
        purpose,
    }
}

fn aliases(list: &str) -> Vec<(String, String)> {
    let inner = list.split(['{', '}']).nth(1).unwrap_or(list);
    inner
        .split(',')
        .filter_map(|pair| pair.trim().trim_end_matches(';').split_once(" as "))
        .filter_map(|(source, alias)| {
            let source = source.rsplit("::").next()?.trim();
            Some((alias.trim().to_owned(), source.to_owned()))
        })
        .collect()
}

fn item_name(line: &str) -> Option<&str> {
    let rest = line
        .strip_prefix("pub fn ")
        .or_else(|| line.strip_prefix("pub const fn "))
        .or_else(|| line.strip_prefix("pub struct "))?;
    rest.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .next()
}

fn builder_type(signature: &str) -> Option<&str> {
    let (_, output) = signature.rsplit_once("->")?;
    let path = output.trim().split('<').next()?;
    let kind = path.rsplit("::").next()?.trim();
    (kind.ends_with("Builder") && kind != "NodeBuilder").then_some(kind)
}

pub(crate) fn demo_source(file: &str, build: &str) -> Option<String> {
    function_source(file, &format!("{}_demo", build.replace('-', "_")))
}

pub(crate) fn function_source(file: &str, name: &str) -> Option<String> {
    let head = format!("fn {name}(");
    let start = [format!("\n{head}"), format!("\npub(crate) {head}")]
        .iter()
        .find_map(|line| {
            file.find(line.as_str())
                .map(|at| at + line.len() - head.len())
        })?;
    let body = file.get(start..)?;
    let open = body.find('{')?;
    let mut depth = 0_usize;
    for (offset, ch) in body.get(open..)?.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return body.get(..=open + offset).map(str::to_owned);
                }
            }
            _ => {}
        }
    }
    None
}
