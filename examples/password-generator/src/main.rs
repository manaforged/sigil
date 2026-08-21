//! Dogfood example — a tier-1 utility password generator.
//!
//! Reactive state:
//! - `length: Signal<f32>` — slider value, 4..=64 (f32 because `slider`
//!   takes `Signal<f32>`; see FRICTION.md "Confusing names" for why).
//! - `use_lower / use_upper / use_digits / use_symbols: Signal<bool>` —
//!   per-class toggles wired to checkboxes.
//! - `seed: Signal<u64>` — bumped by the "Generate" button so the
//!   derived `password` re-runs and produces a new value.
//! - `password: Signal<String>` — derived; recomputes whenever any
//!   input changes or the user presses Generate.
//!
//! Layout: a vertical column with a big monospaced password readout at
//! the top, the length slider with a live count, four class toggles,
//! and a Generate button that bumps the seed.

// Examples bend the f32->integer cast pedantic lints: slider values are
// bounded f32s that always round to small positive integers, so the
// "could truncate" / "could lose sign" warnings are noise here.
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use std::time::{SystemTime, UNIX_EPOCH};

use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::launch(("Sigil - password-generator", 520, 460), app_tree)
}

fn app_tree(cx: &mut Cx) -> NodeId {
    // Reactive inputs.
    let length: Signal<f32> = signal(16.0_f32);
    let use_lower: Signal<bool> = signal(true);
    let use_upper: Signal<bool> = signal(true);
    let use_digits: Signal<bool> = signal(true);
    let use_symbols: Signal<bool> = signal(false);
    let seed: Signal<u64> = signal(initial_seed());

    // Derived: the generated password reruns whenever any input or the
    // seed changes. Cheap pure function of its inputs, so re-deriving on
    // every keystroke is fine.
    let password: Signal<String> = derived(move || {
        let mut classes = 0_u8;
        if use_lower.get() {
            classes |= CLASS_LOWER;
        }
        if use_upper.get() {
            classes |= CLASS_UPPER;
        }
        if use_digits.get() {
            classes |= CLASS_DIGITS;
        }
        if use_symbols.get() {
            classes |= CLASS_SYMBOLS;
        }
        let spec = PasswordSpec {
            length: length.get().round().clamp(1.0, 256.0) as u32,
            classes,
        };
        generate(spec, seed.get())
    });

    let length_label = derived(move || format!("length: {}", length.get().round() as u32));

    cx.add(
        container()
            .layout(
                LayoutStyle::flex_column()
                    .fixed_size(520.0, 460.0)
                    .padding(16.0)
                    .gap(12.0),
            )
            .class(classes!("bg-surface"))
            .child(text(password).class(classes!("text-on-surface text-xl font-mono")))
            .child(text(length_label).class(classes!("text-on-surface text-sm")))
            .child(slider(length, 4.0..=64.0).step(1.0))
            .child(class_toggles(use_lower, use_upper, use_digits, use_symbols))
            .child(button("Generate").primary().activate(move || {
                // Bump the seed; the derived password re-runs.
                seed.set(seed.peek().wrapping_add(0x9E37_79B9_7F4A_7C15));
            })),
    )
}

fn class_toggles(
    use_lower: Signal<bool>,
    use_upper: Signal<bool>,
    use_digits: Signal<bool>,
    use_symbols: Signal<bool>,
) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().gap(4.0))
        .child(checkbox(use_lower).label("lowercase (a-z)"))
        .child(checkbox(use_upper).label("UPPERCASE (A-Z)"))
        .child(checkbox(use_digits).label("digits (0-9)"))
        .child(checkbox(use_symbols).label("symbols (!@#…)"))
}

// --- pure logic --------------------------------------------------------

const CLASS_LOWER: u8 = 0b0001;
const CLASS_UPPER: u8 = 0b0010;
const CLASS_DIGITS: u8 = 0b0100;
const CLASS_SYMBOLS: u8 = 0b1000;

const LOWER: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
const UPPER: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIGITS: &[u8] = b"0123456789";
const SYMBOLS: &[u8] = b"!@#$%^&*()-_=+[]{};:,.?";

#[derive(Clone, Copy, Debug)]
struct PasswordSpec {
    length: u32,
    classes: u8,
}

fn alphabet(classes: u8) -> Vec<u8> {
    let mut out = Vec::with_capacity(96);
    if classes & CLASS_LOWER != 0 {
        out.extend_from_slice(LOWER);
    }
    if classes & CLASS_UPPER != 0 {
        out.extend_from_slice(UPPER);
    }
    if classes & CLASS_DIGITS != 0 {
        out.extend_from_slice(DIGITS);
    }
    if classes & CLASS_SYMBOLS != 0 {
        out.extend_from_slice(SYMBOLS);
    }
    out
}

fn generate(spec: PasswordSpec, seed: u64) -> String {
    let alpha = alphabet(spec.classes);
    if alpha.is_empty() {
        return String::from("(pick at least one class)");
    }
    let length = spec.length as usize;
    let mut state = seed.max(1);
    let mut out = String::with_capacity(length);
    let alpha_len = alpha.len() as u64;
    for _ in 0..length {
        state = xorshift64(state);
        let pick = usize::try_from(state % alpha_len).unwrap_or(0);
        let byte = alpha.get(pick).copied().unwrap_or(b'?');
        out.push(byte as char);
    }
    out
}

const fn xorshift64(mut x: u64) -> u64 {
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    x
}

fn initial_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0x1234_5678_9ABC_DEF0, |d| {
            let nanos = u64::try_from(d.as_nanos() & u128::from(u64::MAX)).unwrap_or(1);
            nanos.max(1)
        })
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
