use std::time::Instant;

use crate::profile::store;

pub struct Span {
    name: &'static str,
    cat: &'static str,
    start_us: Option<u64>,
    start: Option<Instant>,
}

impl Span {
    pub(crate) fn new(name: &'static str) -> Self {
        Self::in_category(category(name), name)
    }

    pub(crate) fn in_category(cat: &'static str, name: &'static str) -> Self {
        if store::enabled() {
            Self {
                name,
                cat,
                start_us: Some(store::now_us()),
                start: Some(Instant::now()),
            }
        } else {
            Self {
                name,
                cat,
                start_us: None,
                start: None,
            }
        }
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        let (Some(start_us), Some(start)) = (self.start_us.take(), self.start.take()) else {
            return;
        };
        store::complete(
            self.name,
            self.cat,
            start_us,
            u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX),
        );
    }
}

fn category(name: &str) -> &'static str {
    if name.starts_with("layout") {
        "layout"
    } else if name == "emit" || name == "paint" {
        "paint"
    } else if name == "gpu" {
        "gpu"
    } else if name == "a11y" {
        "a11y"
    } else if name == "effect" {
        "reactive"
    } else {
        "app"
    }
}
