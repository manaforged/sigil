use std::collections::{HashMap, HashSet};

use pulldown_cmark::{BrokenLink, CowStr, RefDefs};

#[derive(Clone, Debug)]
pub(super) struct Def {
    dest: String,
    title: String,
}

#[derive(Default)]
pub(super) struct Refs {
    defs: HashMap<String, Def>,
    broken: HashSet<String>,
}

#[derive(Default)]
pub(super) struct Report {
    defs: Vec<(String, Def, usize)>,
    broken: Vec<(String, usize)>,
}

pub(super) fn label(raw: &str) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

impl Refs {
    pub(super) fn resolve<'s>(
        &self,
        link: &BrokenLink<'_>,
        broken: &mut Vec<(String, usize)>,
    ) -> Option<(CowStr<'s>, CowStr<'s>)> {
        let key = label(&link.reference);
        if let Some(def) = self.defs.get(&key) {
            return Some((def.dest.clone().into(), def.title.clone().into()));
        }
        broken.push((key, link.span.start));
        None
    }

    pub(super) fn wanted_by(&self, report: &Report) -> bool {
        report
            .defs
            .iter()
            .any(|(key, _, _)| self.broken.contains(key))
    }

    pub(super) fn commit(&mut self, report: &Report, end: usize) {
        for (key, def, at) in &report.defs {
            if *at < end {
                self.defs.entry(key.clone()).or_insert_with(|| def.clone());
            }
        }
        for (key, at) in &report.broken {
            if *at < end {
                self.broken.insert(key.clone());
            }
        }
    }
}

pub(super) type Found = Vec<(String, Def, usize)>;

pub(super) fn found(defs: &RefDefs<'_>) -> Found {
    defs.iter()
        .map(|(key, def)| {
            let owned = Def {
                dest: def.dest.to_string(),
                title: def.title.as_deref().unwrap_or_default().to_string(),
            };
            (label(key), owned, def.span.start)
        })
        .collect()
}

impl Report {
    pub(super) const fn new(defs: Found, broken: Vec<(String, usize)>) -> Self {
        Self { defs, broken }
    }
}
