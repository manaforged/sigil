use serde_json::{Value, json};

const API_DOC: &str = include_str!("../../../../docs/api.md");
const SECTION: &str = "\n## Prelude";

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
}

impl ApiMap {
    pub(crate) fn load() -> Self {
        let start = API_DOC.find(SECTION).unwrap_or(0);
        let rows = API_DOC
            .get(start..)
            .unwrap_or(API_DOC)
            .lines()
            .filter_map(parse_row)
            .collect();
        Self { rows }
    }

    pub(crate) fn row(&self, name: &str) -> Option<&ApiRow> {
        self.rows.iter().find(|row| row.name == name)
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
}

fn parse_row(line: &str) -> Option<ApiRow> {
    let inner = line.strip_prefix("| ")?.strip_suffix(" |")?;
    if !inner.starts_with('`') {
        return None;
    }
    let cells: Vec<&str> = inner.split(" | ").collect();
    let (name, rest) = cells.split_first()?;
    let (purpose, signature) = rest.split_last()?;
    if signature.is_empty() {
        return None;
    }
    let joined = signature.join(" | ");
    Some(ApiRow {
        name: unquote(name).to_owned(),
        signature: unquote(&joined).to_owned(),
        purpose: (*purpose).to_owned(),
    })
}

fn unquote(cell: &str) -> &str {
    cell.strip_prefix('`')
        .and_then(|rest| rest.strip_suffix('`'))
        .filter(|rest| !rest.contains('`'))
        .unwrap_or(cell)
}

fn builder_type(signature: &str) -> Option<&str> {
    let (_, output) = signature.rsplit_once("->")?;
    let path = output.trim().split('<').next()?;
    let kind = path.rsplit("::").next()?.trim();
    (kind.ends_with("Builder") && kind != "NodeBuilder").then_some(kind)
}

pub(crate) fn function_source(file: &str, name: &str) -> Option<String> {
    let head = format!("\nfn {name}(");
    let start = file.find(&head)? + 1;
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
