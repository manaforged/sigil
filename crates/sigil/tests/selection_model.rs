use std::collections::BTreeSet;

use sigil::widgets::{Selection, SelectionMode};

const ROWS: usize = 4;
const DEPTH: u32 = 5;

#[derive(Clone, Copy, Debug)]
enum Op {
    Select(usize),
    Toggle(usize),
    Extend(usize),
    Clear,
}

#[derive(Default)]
struct Model {
    rows: BTreeSet<usize>,
    anchor: Option<usize>,
}

impl Model {
    fn only(&mut self, row: usize) {
        self.rows = BTreeSet::from([row]);
        self.anchor = Some(row);
    }

    fn apply(&mut self, mode: SelectionMode, op: Op) {
        if mode == SelectionMode::None {
            return;
        }
        let many = matches!(mode, SelectionMode::Multi | SelectionMode::Extend);
        match op {
            Op::Clear => *self = Self::default(),
            Op::Toggle(row) if many => {
                if !self.rows.remove(&row) {
                    self.rows.insert(row);
                }
                self.anchor = Some(row);
            }
            Op::Extend(row) if mode == SelectionMode::Extend => {
                let from = self.anchor.unwrap_or(row);
                self.rows = (from.min(row)..=from.max(row)).collect();
                self.anchor = Some(from);
            }
            Op::Select(row) | Op::Toggle(row) | Op::Extend(row) => self.only(row),
        }
    }
}

fn ops() -> Vec<Op> {
    let mut ops = vec![Op::Clear];
    for row in 0..ROWS {
        ops.extend([Op::Select(row), Op::Toggle(row), Op::Extend(row)]);
    }
    ops
}

fn run(selection: &mut Selection, op: Op) {
    match op {
        Op::Select(row) => selection.select(row),
        Op::Toggle(row) => selection.toggle(row),
        Op::Extend(row) => selection.extend(row),
        Op::Clear => selection.clear(),
    }
}

#[test]
fn every_click_sequence_up_to_five_steps_matches_the_selection_rules() {
    let ops = ops();
    let modes = [
        SelectionMode::None,
        SelectionMode::Single,
        SelectionMode::Multi,
        SelectionMode::Extend,
    ];
    for mode in modes {
        for len in 1..=DEPTH {
            for mut index in 0..ops.len().pow(len) {
                let mut selection = Selection::new(mode);
                let mut model = Model::default();
                let mut steps = Vec::new();
                for _ in 0..len {
                    let op = ops[index % ops.len()];
                    index /= ops.len();
                    steps.push(op);
                    run(&mut selection, op);
                    model.apply(mode, op);
                    let shown: BTreeSet<usize> = selection.indices().collect();
                    assert_eq!(shown, model.rows, "{mode:?} after {steps:?}");
                    for row in 0..ROWS {
                        assert_eq!(selection.contains(row), model.rows.contains(&row));
                    }
                    if mode == SelectionMode::Single {
                        assert!(shown.len() <= 1, "{mode:?} after {steps:?}");
                    }
                }
            }
        }
    }
}
