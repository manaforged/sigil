use std::path::{Path, PathBuf};

use crate::budgets::Budgets;

const DEFAULT_DIR: &str = "../../target/criterion";

fn estimate(dir: &Path, id: &str) -> Option<f64> {
    let path = dir.join(id).join("new").join("estimates.json");
    let text = std::fs::read_to_string(path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    json.get("median")?.get("point_estimate")?.as_f64()
}

pub(crate) fn check(table: &Budgets, dir: Option<&str>) -> bool {
    let dir = dir.map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join(DEFAULT_DIR),
        PathBuf::from,
    );
    let limit = 1.0 + table.bench_tolerance;
    let mut ok = true;
    println!("bench  baseline_ns  median_ns  ratio");
    for (id, budget) in &table.benches {
        let Some(median) = estimate(&dir, id) else {
            println!("{id}  missing criterion estimate under {}", dir.display());
            ok = false;
            continue;
        };
        let ratio = median / budget.baseline_ns;
        let pass = ratio <= limit;
        ok &= pass;
        println!(
            "{id}  {:.0}  {median:.0}  {ratio:.2}  {}",
            budget.baseline_ns,
            if pass { "ok" } else { "REGRESSED" }
        );
    }
    ok
}
