//! Proc macros for Sigil typed actions.
//!
//! This crate ships `#[derive(Action)]` — the one-line authoring shortcut
//! for declaring an action type that can flow through
//! `sigil_actions::ActionRegistry`.
//!
//! See **DR-046 §1** for the full Action trait design. Layer A (this
//! crate + `sigil-actions`) lands the trait + derive + registry +
//! dispatch loop. Layer B wires the focus/Context surface on top.
//!
//! **Resolution contract:** the derive expands to paths under
//! `sigil::__private::…`. Consumers must depend on the `sigil-ui` facade
//! crate (library name `sigil`) and must not rename that dependency, or
//! the expansion will not resolve.
//!
//! ```rust,ignore
//! use sigil_actions::Action;
//!
//! #[derive(Action)]
//! struct CloseTab;
//!
//! assert_eq!(CloseTab.debug_name(), "CloseTab");
//! ```

#![deny(missing_docs)]
use proc_macro::TokenStream;
use proc_macro2 as _;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

#[cfg(test)]
use sigil as _;
#[cfg(test)]
use sigil_actions as _;

/// Derive `sigil_actions::Action` for a type.
///
/// Generated impl provides:
///
/// - `fn debug_name(&self) -> &'static str` returning the type's
///   identifier as a string literal.
/// - `fn boxed_clone(&self) -> Box<dyn Action>` that delegates to
///   `Clone::clone` — used by the deferred-replay path in
///   `sigil-reactive`.
/// - `fn as_any(&self) -> &dyn Any` for the `dispatch_dyn` downcast
///   path.
///
/// The `boxed_clone` impl requires the deriving type to also
/// implement `Clone`. Write `#[derive(Clone, Action)]` together — the
/// macro can't emit `impl Clone` itself because it would conflict
/// with user-written `Clone` or `Copy` derives.
#[proc_macro_derive(Action)]
pub fn derive_action(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let name_str = name.to_string();
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let expanded = quote! {
        impl #impl_generics ::sigil::__private::actions::Action for #name #ty_generics #where_clause {
            fn debug_name(&self) -> &'static str {
                #name_str
            }
            fn boxed_clone(&self) -> ::std::boxed::Box<dyn ::sigil::__private::actions::Action> {
                ::std::boxed::Box::new(::core::clone::Clone::clone(self))
            }
            fn as_any(&self) -> &dyn ::core::any::Any {
                self
            }
        }
    };

    TokenStream::from(expanded)
}
