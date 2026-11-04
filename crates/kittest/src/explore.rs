use sigil::prelude::IntoNode;

use crate::Harness;
use crate::harness::fail;
use crate::scene_gen::Entropy;

/// One named user step: it drives the app and applies the same change to the model.
pub type Step<M> = (&'static str, fn(&mut Harness, &mut M));

impl Harness {
    /// Runs every sequence of `steps` up to `depth`, shortest first, on a fresh app and model, and panics with the shortest sequence that fails `check`.
    #[track_caller]
    pub fn explore<M: Default>(
        app: impl IntoNode + Clone + 'static,
        steps: &[Step<M>],
        depth: u32,
        check: impl Fn(&mut Self, &M) -> Result<(), String>,
    ) {
        let count = u64::try_from(steps.len()).unwrap_or(u64::MAX);
        if count == 0 {
            fail("explore needs at least one step");
        }
        for len in 1..=depth {
            let total = count
                .checked_pow(len)
                .unwrap_or_else(|| fail(format!("explore: {count}^{len} sequences overflow")));
            for mut index in 0..total {
                let order: Vec<usize> = (0..len)
                    .map(|_| {
                        let step = index % count;
                        index /= count;
                        usize::try_from(step).unwrap_or(0)
                    })
                    .collect();
                replay(&app, steps, &order, &check, None);
            }
        }
    }

    /// Runs one sequence of `length` steps picked from `seed` on a fresh app and model, and panics at the first step that fails `check`.
    #[track_caller]
    pub fn walk<M: Default>(
        app: impl IntoNode + Clone + 'static,
        steps: &[Step<M>],
        length: usize,
        seed: u64,
        check: impl Fn(&mut Self, &M) -> Result<(), String>,
    ) {
        if steps.is_empty() {
            fail("walk needs at least one step");
        }
        let count = u32::try_from(steps.len()).unwrap_or(u32::MAX);
        let mut entropy = Entropy::from_seed(seed);
        let order: Vec<usize> = (0..length)
            .map(|_| usize::try_from(entropy.below(count)).unwrap_or(0))
            .collect();
        replay(&app, steps, &order, &check, Some(seed));
    }
}

#[track_caller]
fn replay<M: Default>(
    app: &(impl IntoNode + Clone + 'static),
    steps: &[Step<M>],
    order: &[usize],
    check: &impl Fn(&mut Harness, &M) -> Result<(), String>,
    seed: Option<u64>,
) {
    let mut harness = Harness::new(app.clone());
    let mut model = M::default();
    harness.settle();
    let broke = |at: usize, why: String| -> ! {
        let names: Vec<&str> = order[..at].iter().map(|&step| steps[step].0).collect();
        let path = if names.is_empty() {
            "start".to_owned()
        } else {
            names.join(" -> ")
        };
        let seed = seed.map_or_else(String::new, |seed| format!(" (seed {seed})"));
        fail(format!("{path} broke the check{seed}: {why}"))
    };
    if let Err(why) = check(&mut harness, &model) {
        broke(0, why);
    }
    for (at, &step) in order.iter().enumerate() {
        (steps[step].1)(&mut harness, &mut model);
        if let Err(why) = check(&mut harness, &model) {
            broke(at + 1, why);
        }
    }
}

#[cfg(test)]
#[path = "explore_tests.rs"]
mod tests;
