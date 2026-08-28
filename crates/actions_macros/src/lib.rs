use proc_macro::TokenStream;
use proc_macro2 as _;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

#[cfg(test)]
use sigil as _;
#[cfg(test)]
use sigil_actions as _;

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
