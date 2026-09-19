use crate::layout::flex_lines::{FlexItem, FlexLine};
use crate::layout::resolve::clamp;

pub(super) fn resolve_flexible(line: &mut FlexLine, container_main: f32) {
    let used_basis: f32 = line.items.iter().map(FlexItem::outer_main_basis).sum();
    let gap_total = line.gap * (line.items.len().saturating_sub(1) as f32);
    let free = container_main - used_basis - gap_total;
    if free.abs() < f32::EPSILON || line.items.is_empty() || container_main <= f32::EPSILON {
        for item in &mut line.items {
            item.main_size = clamp_main(item, item.flex_basis_resolved);
        }
        return;
    }
    let growing = free > 0.0;
    let mut frozen = freeze_inflexible(&mut line.items, growing);
    while frozen.contains(&false) {
        let remaining = remaining_free(&line.items, &frozen, container_main, gap_total);
        if !flex_round(&mut line.items, &mut frozen, remaining, growing) {
            break;
        }
    }
}

fn flex_factor(item: &FlexItem, growing: bool) -> f32 {
    if growing {
        item.flex_grow.max(0.0)
    } else {
        item.flex_shrink.max(0.0) * item.flex_basis_resolved
    }
}

fn freeze_inflexible(items: &mut [FlexItem], growing: bool) -> Vec<bool> {
    items
        .iter_mut()
        .map(|item| {
            let inflexible = flex_factor(item, growing) <= f32::EPSILON;
            item.main_size = if inflexible {
                clamp_main(item, item.flex_basis_resolved)
            } else {
                item.flex_basis_resolved
            };
            inflexible
        })
        .collect()
}

fn remaining_free(items: &[FlexItem], frozen: &[bool], container_main: f32, gap_total: f32) -> f32 {
    let used: f32 = items
        .iter()
        .zip(frozen)
        .map(|(item, &is_frozen)| {
            let base = if is_frozen {
                item.main_size
            } else {
                item.flex_basis_resolved
            };
            base + item.margin_main_start() + item.margin_main_end()
        })
        .sum();
    container_main - used - gap_total
}

fn flex_round(items: &mut [FlexItem], frozen: &mut [bool], remaining: f32, growing: bool) -> bool {
    let total: f32 = items
        .iter()
        .zip(frozen.iter())
        .filter(|(_, f)| !**f)
        .map(|(item, _)| flex_factor(item, growing))
        .sum();
    if total <= f32::EPSILON {
        for (item, is_frozen) in items.iter_mut().zip(frozen.iter_mut()) {
            if !*is_frozen {
                item.main_size = clamp_main(item, item.flex_basis_resolved);
                *is_frozen = true;
            }
        }
        return false;
    }
    let mut violations = vec![0.0_f32; items.len()];
    let mut net = 0.0_f32;
    for ((item, &is_frozen), violation) in items
        .iter_mut()
        .zip(frozen.iter())
        .zip(violations.iter_mut())
    {
        if is_frozen {
            continue;
        }
        let target = item.flex_basis_resolved + remaining * (flex_factor(item, growing) / total);
        item.main_size = clamp_main(item, target);
        *violation = item.main_size - target;
        net += *violation;
    }
    freeze_violations(frozen, &violations, net)
}

fn freeze_violations(frozen: &mut [bool], violations: &[f32], net: f32) -> bool {
    let mut progressed = false;
    for (is_frozen, &violation) in frozen.iter_mut().zip(violations) {
        if *is_frozen {
            continue;
        }
        let freeze = if net.abs() < f32::EPSILON {
            true
        } else if net > 0.0 {
            violation > 0.0
        } else {
            violation < 0.0
        };
        if freeze {
            *is_frozen = true;
            progressed = true;
        }
    }
    if !progressed {
        for f in frozen.iter_mut() {
            *f = true;
        }
    }
    progressed
}

fn clamp_main(item: &FlexItem, value: f32) -> f32 {
    clamp(value, item.min_main, item.max_main).max(0.0)
}
