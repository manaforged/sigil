use std::collections::BTreeMap;

use serde::Deserialize;

const TABLE: &str = include_str!("../../perf_budgets.toml");

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Scenario {
    Idle,
    Hover,
    Scroll,
    Typing,
    Chart,
    Terminal,
}

#[derive(Deserialize)]
pub(crate) struct FrameBudget {
    pub allocs: u64,
}

#[derive(Deserialize)]
pub(crate) struct BenchBudget {
    pub baseline_ns: f64,
}

#[derive(Deserialize)]
pub(crate) struct Budgets {
    pub bench_tolerance: f64,
    pub frames: BTreeMap<Scenario, FrameBudget>,
    pub benches: BTreeMap<String, BenchBudget>,
}

pub(crate) fn load() -> Budgets {
    match toml::from_str(TABLE) {
        Ok(budgets) => budgets,
        Err(error) => panic!("perf_budgets.toml: {error}"),
    }
}
