use std::collections::BTreeMap;
use std::fmt::Write as _;

use super::{CHANNEL_TOLERANCE, MAX_OFF_FRACTION, SUITE_TAG, Tally};

pub(super) fn report(tallies: &BTreeMap<String, Tally>) -> String {
    let mut out = format!(
        "resvg suite {SUITE_TAG}; pass = at most {:.0}% of pixels off by more than {CHANNEL_TOLERANCE}/255 in a channel, both composited over white\n",
        MAX_OFF_FRACTION * 100.0
    );
    out.push_str(
        "category\ttests\traster_pass\tvector_pass\tgpu_tests\tgpu_vector_pass\tfallbacks\n",
    );
    let mut totals = [0usize; 5];
    for (category, tally) in tallies {
        let fallbacks: Vec<String> = tally
            .fallbacks
            .iter()
            .map(|(reason, count)| format!("{reason}={count}"))
            .collect();
        let _ = writeln!(
            out,
            "{category}\t{}\t{}\t{}\t{}\t{}\t{}",
            tally.tests,
            tally.raster,
            tally.vector,
            tally.gpu_tests,
            tally.gpu_vector,
            fallbacks.join(",")
        );
        for (total, value) in totals.iter_mut().zip([
            tally.tests,
            tally.raster,
            tally.vector,
            tally.gpu_tests,
            tally.gpu_vector,
        ]) {
            *total += value;
        }
    }
    let [tests, raster, vector, gpu_tests, gpu_vector] = totals;
    let _ = writeln!(
        out,
        "total\t{tests}\t{raster}\t{vector}\t{gpu_tests}\t{gpu_vector}\t"
    );
    out
}
