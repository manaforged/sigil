//! Integration tests for the `Signal<T>` operator overloads and
//! `Display` impl introduced in the ergonomics polish.
//!
//! See `crates/sigil/reactive/src/signal_ops.rs`.

#![allow(missing_docs)]

// `criterion` is a dev-dep used only by benches; alias to silence the
// `unused_crate_dependencies` lint for this integration-test binary.
// `smallvec` is a regular dep but unused here — same treatment.
use criterion as _;
use smallvec as _;

use std::cell::Cell;
use std::rc::Rc;

use sigil_reactive::{Runtime, effect, signal};

#[test]
fn add_assign_mutates_and_fires_effects() {
    Runtime::scope("add_assign_mutates_and_fires_effects", || {
        let count = signal(0_i32);
        let seen = Rc::new(Cell::new(0_i32));

        let seen_clone = Rc::clone(&seen);
        effect(move || {
            seen_clone.set(count.get());
        });
        assert_eq!(seen.get(), 0);

        let mut count = count;
        count += 1;
        assert_eq!(seen.get(), 1);
        count += 4;
        assert_eq!(seen.get(), 5);
    });
}

#[test]
fn sub_assign_mutates() {
    Runtime::scope("sub_assign_mutates", || {
        let mut n = signal(10_i32);
        n -= 3;
        assert_eq!(n.peek(), 7);
        n -= 7;
        assert_eq!(n.peek(), 0);
    });
}

#[test]
fn mul_div_rem_assign() {
    Runtime::scope("mul_div_rem_assign", || {
        let mut n = signal(6_i32);
        n *= 7;
        assert_eq!(n.peek(), 42);
        n /= 6;
        assert_eq!(n.peek(), 7);
        n %= 3;
        assert_eq!(n.peek(), 1);
    });
}

#[test]
fn bitops_assign() {
    Runtime::scope("bitops_assign", || {
        let mut bits = signal(0b1010_u8);
        bits |= 0b0001;
        assert_eq!(bits.peek(), 0b1011);
        bits &= 0b1110;
        assert_eq!(bits.peek(), 0b1010);
        bits ^= 0b1111;
        assert_eq!(bits.peek(), 0b0101);
    });
}

#[test]
fn shift_assign() {
    Runtime::scope("shift_assign", || {
        let mut n = signal(1_u32);
        n <<= 4;
        assert_eq!(n.peek(), 16);
        n >>= 2;
        assert_eq!(n.peek(), 4);
    });
}

/// Shift ops must accept an RHS type different from `T` — `u32`
/// implements `ShlAssign<u8>`, `ShlAssign<u16>`, `ShlAssign<usize>`,
/// and so on, and the macro must not collapse to `T = U`.
#[test]
fn shift_assign_with_different_rhs_type() {
    Runtime::scope("shift_assign_with_different_rhs_type", || {
        let mut n = signal(1_u32);
        n <<= 4_u8;
        assert_eq!(n.peek(), 16);
        n >>= 1_u16;
        assert_eq!(n.peek(), 8);
        n <<= 2_usize;
        assert_eq!(n.peek(), 32);
    });
}

#[test]
fn assign_ops_work_inside_move_closure() {
    Runtime::scope("assign_ops_work_inside_move_closure", || {
        let count = signal(0_i32);
        // `Signal: Copy`, so `move` closures capturing it own a fresh
        // mutable copy of the handle — the outer binding need not be
        // `let mut`. This is the on_click ergonomic the polish unlocks.
        let increment = move |amount: i32| {
            let mut c = count;
            c += amount;
        };
        increment(3);
        increment(4);
        assert_eq!(count.peek(), 7);
    });
}

#[test]
fn display_reads_inner_value() {
    Runtime::scope("display_reads_inner_value", || {
        let n = signal(42_i32);
        assert_eq!(format!("{n}"), "42");
        assert_eq!(format!("Count: {n}"), "Count: 42");

        let s = signal(String::from("hello"));
        assert_eq!(format!("{s}"), "hello");
    });
}

#[test]
fn display_inside_effect_subscribes() {
    Runtime::scope("display_inside_effect_subscribes", || {
        let count = signal(0_i32);
        let rendered = Rc::new(Cell::new(String::new()));

        let rendered_clone = Rc::clone(&rendered);
        effect(move || {
            // `format!("{count}")` calls Signal's Display impl, which
            // forwards to .get() and subscribes this effect.
            rendered_clone.set(format!("Count: {count}"));
        });
        assert_eq!(rendered.take(), "Count: 0");

        count.set(7);
        assert_eq!(rendered.take(), "Count: 7");

        count.set(123);
        assert_eq!(rendered.take(), "Count: 123");
    });
}

#[test]
fn assign_op_respects_recursion_guard() {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    Runtime::scope("assign_op_respects_recursion_guard", || {
        let mut n = signal(0_i32);

        // Sanity: bare AddAssign works.
        n += 1;
        assert_eq!(n.peek(), 1);

        // Direct recursive `Signal::update` on the same signal must
        // still panic (the guard is upheld through the assign-op
        // forwarding path).
        let result = catch_unwind(AssertUnwindSafe(|| {
            n.update(|_inner| {
                let mut inner = n;
                inner += 1;
            });
        }));
        assert!(
            result.is_err(),
            "expected re-entrant AddAssign on the same signal to panic"
        );
    });
}
