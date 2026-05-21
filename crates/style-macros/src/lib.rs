//! Proc macros for Sigil utility classes.

use proc_macro::TokenStream;
use quote::quote;

#[cfg(test)]
use style as _;
#[cfg(test)]
use trybuild as _;

pub(crate) mod parser;

#[derive(Clone, Copy)]
struct StaticAtom;

mod generated {
    #![allow(clippy::redundant_pub_crate)]
    #![allow(clippy::unreadable_literal)]

    use super::StaticAtom;

    include!(concat!(env!("OUT_DIR"), "/structural_table.rs"));
}

/// Resolve utility class literals into a static style-atom slice.
#[proc_macro]
pub fn classes(input: TokenStream) -> TokenStream {
    let _table_len = generated::STRUCTURAL_TABLE.len();
    let input = proc_macro2::TokenStream::from(input);
    match parser::parse_segments(input) {
        Ok(segments) => emit_segments(&segments, Vec::new()).into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn emit_segments(
    segments: &[parser::Segment],
    atoms: Vec<proc_macro2::TokenStream>,
) -> proc_macro2::TokenStream {
    let Some((head, tail)) = segments.split_first() else {
        return quote!(&[#(#atoms),*] as &[::style::StyleAtom]);
    };
    match head {
        parser::Segment::Static(static_atoms) => {
            let mut next = atoms;
            next.extend(static_atoms.iter().cloned());
            emit_segments(tail, next)
        }
        parser::Segment::Match(match_segment) => {
            let expr = &match_segment.expr;
            let arms = match_segment.arms.iter().map(|arm| {
                let pat = &arm.pat;
                let guard = arm
                    .guard
                    .as_ref()
                    .map(|(if_token, guard)| quote!(#if_token #guard));
                let mut next = atoms.clone();
                next.extend(arm.atoms.iter().cloned());
                let body = emit_segments(tail, next);
                quote!(#pat #guard => #body)
            });
            quote!(match #expr { #(#arms),* })
        }
    }
}
