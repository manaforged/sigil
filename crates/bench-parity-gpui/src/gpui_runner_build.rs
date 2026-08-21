use super::{
    EquivalenceCheck, HashMap, Host, HostInfo, OutputProof, Range, Workload, new_host, pass_if,
    time_construct_layout, visible_band,
};

#[allow(
    clippy::unnecessary_wraps,
    clippy::redundant_closure,
    reason = "callers stay Result for the runner trait"
)]
pub(super) fn sample_build(
    workload: &Workload,
    host: &mut Host,
    mut build: impl FnMut() -> gpui::AnyElement,
) -> Result<bench_parity::RunResult, String> {
    let p = &workload.params;
    let (samples_ns, root_size) = measure_build(host, p, &mut build);
    let extra = build_verdicts(workload, samples_ns.is_empty());
    let output = build_output(workload, root_size);
    Ok(finish(
        workload,
        samples_ns,
        extra,
        HashMap::new(),
        Some(output),
    ))
}

fn measure_build(
    host: &mut Host,
    p: &bench_parity::WorkloadParams,
    build: &mut impl FnMut() -> gpui::AnyElement,
) -> (Vec<u64>, [i64; 2]) {
    let surface = p.surface();
    let samples = {
        let mut run = || {
            *host = new_host();
            time_construct_layout(host, surface, p, &mut *build).0
        };
        for _ in 0..p.warmup {
            let _ = run();
        }
        (0..p.iterations).map(|_| run()).collect()
    };
    let (_, _, root_size) = time_construct_layout(host, surface, p, &mut *build);
    (samples, root_size)
}

fn build_verdicts(workload: &Workload, empty: bool) -> HashMap<String, EquivalenceCheck> {
    let mut extra = HashMap::new();
    let verdict = || pass_if(!empty, "no samples");
    extra.insert("layout_did_work".into(), verdict());
    if workload.category == "text-labels" {
        extra.insert("text_did_work".into(), verdict());
    }
    extra
}

fn build_output(workload: &Workload, root_size: [i64; 2]) -> OutputProof {
    let p = &workload.params;
    let mut values = std::collections::BTreeMap::new();
    values.insert("rows".into(), u64::try_from(p.row_count).unwrap_or(0));
    let real_text = workload.category == "text-labels";
    if !real_text {
        values.insert(
            "cells_per_row".into(),
            u64::try_from(p.cells()).unwrap_or(0),
        );
    }
    let text_rows = real_text.then(|| {
        vec![
            (0..p.row_count)
                .map(|index| format!("row {index} label payload"))
                .collect(),
        ]
    });
    OutputProof {
        schema: if real_text {
            "text-stack-v1".into()
        } else {
            "flex-shape-v1".into()
        },
        root_size_milli_px: root_size,
        values,
        text_rows: text_rows.unwrap_or_default(),
    }
}

pub(super) fn last_real_range(ranges: &[Range<usize>]) -> Option<Range<usize>> {
    ranges.iter().rev().find(|r| r.end > r.start + 1).cloned()
}

pub(super) fn visible_check(
    p: &bench_parity::WorkloadParams,
    last: Option<Range<usize>>,
) -> HashMap<String, EquivalenceCheck> {
    let (_, vh) = p.viewport();
    let (lo, hi) = visible_band(vh, p.row_h(), p.row_count);
    let mut out = HashMap::new();
    let verdict = last.map_or_else(
        || EquivalenceCheck::Fail {
            detail: "uniform_list never reported a visible range".into(),
        },
        |range| {
            let actual = range.end - range.start;
            if actual >= lo && actual <= hi {
                EquivalenceCheck::Pass
            } else {
                EquivalenceCheck::Fail {
                    detail: format!("visible rows {actual} not in {lo}..={hi}"),
                }
            }
        },
    );
    out.insert("visible_window_indices".into(), verdict);
    out
}

pub(super) fn verify_visible(visible: usize) -> HashMap<String, u64> {
    let mut out = HashMap::new();
    out.insert("visible_last".into(), u64::try_from(visible).unwrap_or(0));
    out
}

pub(super) fn finish(
    workload: &Workload,
    samples_ns: Vec<u64>,
    extra: HashMap<String, EquivalenceCheck>,
    mut verify: HashMap<String, u64>,
    output: Option<OutputProof>,
) -> bench_parity::RunResult {
    // GPUI builds a fresh element tree every sample. reuse=0 is the
    // work that ran, not the workload's reuse flag.
    verify.insert("reuse".into(), 0);
    verify
        .entry("tick".into())
        .or_insert_with(|| u64::from(workload.params.tick()));
    verify
        .entry("samples".into())
        .or_insert_with(|| u64::try_from(samples_ns.len()).unwrap_or(0));
    let mut checks = extra;
    for spec in &workload.equivalence_checks {
        checks
            .entry(spec.name.clone())
            .or_insert(EquivalenceCheck::NotApplicable);
    }
    eprintln!(
        "VERIFY gpui {} median_us={:.2} visible={}",
        workload.workload_id,
        bench_parity::RunSummary::from_samples(&samples_ns).median_us,
        verify.get("visible_last").copied().unwrap_or(0),
    );
    bench_parity::RunResult {
        framework: "gpui".into(),
        workload_id: workload.workload_id.clone(),
        summary: bench_parity::RunSummary::from_samples(&samples_ns),
        samples_ns,
        equivalence_check_results: checks,
        host: HostInfo::capture(),
        verify,
        output,
        source: Some(bench_parity::SourceInfo::capture()),
    }
}

pub(super) fn validate(p: &bench_parity::WorkloadParams) -> Result<(), String> {
    if p.row_count == 0 {
        return Err("row_count must be > 0".into());
    }
    if p.iterations == 0 {
        return Err("iterations must be > 0".into());
    }
    Ok(())
}
