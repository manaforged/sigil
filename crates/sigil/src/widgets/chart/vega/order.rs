use std::cmp::Ordering;

use super::channel::{Domain, Sort};

pub(super) struct Facts<'a> {
    pub(super) names: &'a [String],
    pub(super) nulls: &'a [bool],
    pub(super) counts: &'a [u32],
    pub(super) totals: &'a [f64],
}

impl Facts<'_> {
    fn natural(&self, a: usize, b: usize) -> Ordering {
        let null = |at: usize| self.nulls.get(at).copied().unwrap_or(false);
        let name = |at: usize| self.names.get(at).map_or("", String::as_str);
        match (null(a), null(b)) {
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            _ => {}
        }
        match (name(a).parse::<f64>(), name(b).parse::<f64>()) {
            (Ok(left), Ok(right)) => left.total_cmp(&right),
            _ => name(a).cmp(name(b)),
        }
    }

    fn count(&self, at: usize) -> u32 {
        self.counts.get(at).copied().unwrap_or(0)
    }

    fn total(&self, at: usize) -> f64 {
        self.totals.get(at).copied().unwrap_or(0.0)
    }
}

fn flip(ordering: Ordering, descending: bool) -> Ordering {
    if descending {
        ordering.reverse()
    } else {
        ordering
    }
}

pub(super) fn rank(facts: &Facts<'_>, sort: &Sort, own: &str) -> Vec<usize> {
    let mut order = (0..facts.names.len()).collect::<Vec<_>>();
    match sort {
        Sort::Unsorted => {}
        Sort::Ascending => order.sort_by(|a, b| facts.natural(*a, *b)),
        Sort::Descending => order.sort_by(|a, b| facts.natural(*b, *a)),
        Sort::Channel(name, descending) if name == own => {
            order.sort_by(|a, b| flip(facts.natural(*a, *b), *descending));
        }
        Sort::Channel(_, descending) => order.sort_by(|a, b| {
            flip(facts.total(*a).total_cmp(&facts.total(*b)), *descending)
        }),
        Sort::Count(descending) => {
            order.sort_by(|a, b| flip(facts.count(*a).cmp(&facts.count(*b)), *descending));
        }
        Sort::Order(listed) => return lead(order, facts.names, listed),
    }
    order
}

fn lead(order: Vec<usize>, names: &[String], listed: &[String]) -> Vec<usize> {
    let at = |index: &usize| {
        names
            .get(*index)
            .and_then(|name| listed.iter().position(|item| item == name))
    };
    let mut first = order.iter().filter(|index| at(index).is_some()).copied().collect::<Vec<_>>();
    first.sort_by_key(|index| at(index));
    first.extend(order.iter().filter(|index| at(index).is_none()));
    first
}

pub(super) fn domain(
    order: Vec<usize>,
    names: &[String],
    domain: &Domain,
    reorder: bool,
) -> (Vec<usize>, Vec<usize>) {
    let listed = match domain {
        Domain::Values(listed) | Domain::Union(listed) => listed.as_slice(),
        Domain::Data | Domain::Span(..) => &[],
    };
    let led = lead(order.clone(), names, listed);
    let fixed = matches!(domain, Domain::Values(_));
    let slot = |index: &usize| {
        let name = names.get(*index);
        let listed_at = name.and_then(|name| listed.iter().position(|item| item == name));
        let led_at = led.iter().position(|item| item == index).unwrap_or(0);
        listed_at.filter(|_| fixed).unwrap_or(led_at)
    };
    let shown = if reorder { led.clone() } else { order };
    let slots = shown.iter().map(slot).collect();
    (shown, slots)
}
