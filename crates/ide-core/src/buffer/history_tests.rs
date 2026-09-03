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
    // h.checkpoint here is intentionally skipped: the editor
    // typically checkpoints on pause; we test undo from the
    // mid-edit state too. But for the "returns first inserted
    // state" assertion, checkpoint then edit then undo:
    h.checkpoint(&current);
    current.insert(11, "!");

    // Undo: rolls back to checkpointed "hello world".
    assert!(h.undo(&mut current));
    assert_eq!(current.to_string_lossy(), "hello world");

    // Undo again: rolls back to "hello".
    assert!(h.undo(&mut current));
    assert_eq!(current.to_string_lossy(), "hello");

    // Undo again: rolls back to baseline "".
    assert!(h.undo(&mut current));
    assert_eq!(current.to_string_lossy(), "");

    // No more undo available.
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

    // Two redos should walk back to "abc".
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

    assert!(h.undo(&mut current)); // "x"
    assert_eq!(h.depth().1, 1); // one redo waiting

    // Branch: new edit + checkpoint should drop the redo entry.
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

    // Push 10 checkpoints — undo stack should never exceed 4.
    for i in 0..10 {
        current.insert(current.len_bytes(), &i.to_string());
        h.checkpoint(&current);
        assert!(h.depth().0 <= 4, "depth overflowed: {:?}", h.depth());
    }
    assert_eq!(h.depth().0, 4);

    // Undo three times. The top of the undo stack is a snapshot
    // of `current` (the just-completed checkpoint), so the first
    // undo's dedup consumes it before popping a distinct prior
    // state. Three visible undos therefore drain four stack
    // slots — leaving the undo stack empty and three entries on
    // the redo stack.
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

// --- Property test ---------------------------------------------

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

    /// Replay the same random op sequence twice — driven by the
    /// same RNG inputs — and confirm we end at the same buffer
    /// state and the same depth pair. Confirms determinism
    /// (no hidden state, no `Arc`-pointer aliasing surprises).
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

    /// After any sequence, undoing K times then redoing K times
    /// returns to the same buffer state. K = min(undo_avail - 1,
    /// remaining budget).
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

        // Undo K times (some may no-op) then redo the same count
        // of actually-applied undos.
        let mut applied = 0usize;
        for _ in 0..k {
            if h.undo(&mut current) { applied += 1; }
        }
        for _ in 0..applied {
            prop_assert!(h.redo(&mut current));
        }
        prop_assert_eq!(current.to_string_lossy(), pre);
    }

    /// The undo stack must never exceed capacity.
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
