use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use smallvec::SmallVec;

use crate::style::{StyleAtom, parse_class_string, parser::ParseError};

pub fn parse_classes(input: &'static str) -> &'static [StyleAtom] {
    static CACHE: OnceLock<Mutex<HashMap<&'static str, &'static [StyleAtom]>>> = OnceLock::new();

    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut cache) = cache.lock() else {
        return parse_and_leak(input);
    };
    if let Some(atoms) = cache.get(input).copied() {
        return atoms;
    }
    let atoms = parse_and_leak(input);
    cache.insert(input, atoms);
    atoms
}

fn parse_and_leak(input: &'static str) -> &'static [StyleAtom] {
    let mut atoms: SmallVec<[StyleAtom; 16]> = SmallVec::new();
    for token in input.split(' ').filter(|token| !token.is_empty()) {
        match parse_class_string(token) {
            Ok(parsed) => atoms.extend(parsed),
            Err(err) => diagnostic::class_parse(input, &err),
        }
    }
    Box::leak(atoms.into_vec().into_boxed_slice())
}

pub(crate) mod diagnostic {
    #![cfg_attr(debug_assertions, allow(clippy::panic))]

    use super::ParseError;
    use crate::style::merge::bump_unknown_class_count;

    #[cfg(debug_assertions)]
    #[cold]
    #[track_caller]
    pub(crate) fn class_parse(input: &str, err: &ParseError) {
        bump_unknown_class_count();
        panic!("invalid class in \"{input}\": {err}");
    }

    #[cfg(not(debug_assertions))]
    #[cold]
    pub(crate) fn class_parse(input: &str, err: &ParseError) {
        use std::collections::HashSet;
        use std::sync::{Mutex, OnceLock};

        static WARNED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
        bump_unknown_class_count();
        let first = WARNED
            .get_or_init(Mutex::default)
            .lock()
            .map_or(true, |mut seen| seen.insert(err.to_string()));
        if first {
            tracing::warn!(
                target: "sigil_style::class",
                input,
                error = %err,
                "unknown class token ignored",
            );
        }
    }
}
