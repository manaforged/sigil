use super::*;
use proptest::prelude::*;

fn make(text: &str) -> Rope {
    Rope::from_str(text)
}

#[test]
fn baseline_only_has_no_undo() {
    let base = make("");
    let mut current = base.clone();
    let mut h = History::new(base);
    assert_eq!(h.depth(), (1, 0));
    assert!(!h.undo(&mut current));
    assert!(!h.redo(&mut current));
    assert_eq!(current.to_string_lossy(), "");
}

#[test]
fn insert_checkpoint_undo_restores_first_state() {
    let base = make("");
    let mut current = base.clone();
    let mut h = History::new(base);

    current.insert(0, "hello");
    h.checkpoint(&current);

    current.insert(5, " world");
    h.checkpoint(&current);
    current.insert(11, "!");

    assert!(h.undo(&mut current));
    assert_eq!(current.to_string_lossy(), "hello world");

    assert!(h.undo(&mut current));
    assert_eq!(current.to_string_lossy(), "hello");

    assert!(h.undo(&mut current));
    assert_eq!(current.to_string_lossy(), "");

    assert!(!h.undo(&mut current));
}

#[test]
fn redo_replays_undone_state() {
    let base = make("a");
    let mut current = base.clone();
    let mut h = History::new(base);

    current.insert(1, "b");
    h.checkpoint(&current);
    current.insert(2, "c");
    h.checkpoint(&current);

    assert!(h.undo(&mut current));
    assert_eq!(current.to_string_lossy(), "ab");
    assert!(h.undo(&mut current));
    assert_eq!(current.to_string_lossy(), "a");

    assert!(h.redo(&mut current));
    assert_eq!(current.to_string_lossy(), "ab");
    assert!(h.redo(&mut current));
    assert_eq!(current.to_string_lossy(), "abc");
    assert!(!h.redo(&mut current));
}

#[test]
fn new_checkpoint_clears_redo() {
    let base = make("");
    let mut current = base.clone();
    let mut h = History::new(base);

    current.insert(0, "x");
    h.checkpoint(&current);
    current.insert(1, "y");
    h.checkpoint(&current);

    assert!(h.undo(&mut current));
    assert_eq!(h.depth().1, 1);

    current.insert(1, "z");
    h.checkpoint(&current);
    assert_eq!(h.depth().1, 0);
    assert!(!h.redo(&mut current));
}

#[test]
fn depth_cap_respected() {
    let base = make("");
    let mut current = base.clone();
    let mut h = History::bounded(base, 4);
    assert_eq!(h.capacity(), 4);

    for i in 0..10 {
        current.insert(current.len_bytes(), &i.to_string());
        h.checkpoint(&current);
        assert!(h.depth().0 <= 4, "depth overflowed: {:?}", h.depth());
    }
    assert_eq!(h.depth().0, 4);

    assert!(h.undo(&mut current));
    assert!(h.undo(&mut current));
    assert!(h.undo(&mut current));
    assert_eq!(h.depth().0, 0);
    assert_eq!(h.depth().1, 3);
}

#[test]
fn bounded_zero_is_clamped_to_one() {
    let h = History::bounded(make(""), 0);
    assert_eq!(h.capacity(), 1);
    assert_eq!(h.depth(), (1, 0));
}

#[test]
fn undo_then_redo_is_identity() {
    let base = make("seed");
    let mut current = base.clone();
    let mut h = History::new(base);

    current.insert(4, "ed");
    h.checkpoint(&current);
    let after_edit = current.to_string_lossy();

    assert!(h.undo(&mut current));
    assert_eq!(current.to_string_lossy(), "seed");
    assert!(h.redo(&mut current));
    assert_eq!(current.to_string_lossy(), after_edit);
}

#[derive(Debug, Clone)]
enum Op {
    Edit(String),
    Checkpoint,
    Undo,
    Redo,
}

fn op_strategy() -> impl Strategy<Value = Op> {
    prop_oneof![
        "[a-z]{1,4}".prop_map(Op::Edit),
        Just(Op::Checkpoint),
        Just(Op::Undo),
        Just(Op::Redo),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        ..ProptestConfig::default()
    })]

    #[test]
    fn replay_is_deterministic(
        ops in prop::collection::vec(op_strategy(), 0..40),
    ) {
        let run = |script: &[Op]| -> (String, (usize, usize)) {
            let base = Rope::from_str("root");
            let mut current = base.clone();
            let mut h = History::new(base);
            for op in script {
                match op {
                    Op::Edit(s) => {
                        let pos = current.len_bytes();
                        current.insert(pos, s);
                    }
                    Op::Checkpoint => h.checkpoint(&current),
                    Op::Undo => { h.undo(&mut current); }
                    Op::Redo => { h.redo(&mut current); }
                }
            }
            (current.to_string_lossy(), h.depth())
        };
        let a = run(&ops);
        let b = run(&ops);
        prop_assert_eq!(a, b);
    }

    #[test]
    fn undo_redo_round_trip(
        ops in prop::collection::vec(op_strategy(), 0..30),
        k in 0usize..10,
    ) {
        let base = Rope::from_str("");
        let mut current = base.clone();
        let mut h = History::new(base);
        for op in &ops {
            match op {
                Op::Edit(s) => {
                    let pos = current.len_bytes();
                    current.insert(pos, s);
                }
                Op::Checkpoint => h.checkpoint(&current),
                Op::Undo => { h.undo(&mut current); }
                Op::Redo => { h.redo(&mut current); }
            }
        }
        let pre = current.to_string_lossy();

        let mut applied = 0usize;
        for _ in 0..k {
            if h.undo(&mut current) { applied += 1; }
        }
        for _ in 0..applied {
            prop_assert!(h.redo(&mut current));
        }
        prop_assert_eq!(current.to_string_lossy(), pre);
    }

    #[test]
    fn cap_holds(
        ops in prop::collection::vec(op_strategy(), 0..200),
        cap in 1usize..8,
    ) {
        let base = Rope::from_str("");
        let mut current = base.clone();
        let mut h = History::bounded(base, cap);
        for op in &ops {
            match op {
                Op::Edit(s) => {
                    let pos = current.len_bytes();
                    current.insert(pos, s);
                }
                Op::Checkpoint => h.checkpoint(&current),
                Op::Undo => { h.undo(&mut current); }
                Op::Redo => { h.redo(&mut current); }
            }
            let (u, r) = h.depth();
            prop_assert!(u <= cap, "undo {} > cap {}", u, cap);
            prop_assert!(r <= cap, "redo {} > cap {}", r, cap);
        }
    }
}
