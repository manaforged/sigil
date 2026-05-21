//! Parser for `classes!` macro inputs.

#![allow(clippy::redundant_pub_crate)]

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{Expr, ExprLit, ExprMatch, Lit, Token};

/// Parsed macro segment.
pub(crate) enum Segment {
    /// Static atoms from a string literal.
    Static(Vec<TokenStream>),
    /// Runtime match whose arms each contribute atoms.
    Match(MatchSegment),
}

/// Parsed match segment.
pub(crate) struct MatchSegment {
    /// Match scrutinee.
    pub(crate) expr: Box<Expr>,
    /// Parsed arms.
    pub(crate) arms: Vec<MatchArm>,
}

/// Parsed match arm.
pub(crate) struct MatchArm {
    /// Arm pattern.
    pub(crate) pat: syn::Pat,
    /// Optional guard.
    pub(crate) guard: Option<(Token![if], Box<Expr>)>,
    /// Atoms contributed by the arm body.
    pub(crate) atoms: Vec<TokenStream>,
}

/// Parse the full macro input.
pub(crate) fn parse_segments(input: TokenStream) -> syn::Result<Vec<Segment>> {
    let parser = Punctuated::<Expr, Token![,]>::parse_terminated;
    let exprs = parser.parse2(input)?;
    exprs.into_iter().map(parse_expr).collect()
}

fn parse_expr(expr: Expr) -> syn::Result<Segment> {
    match expr {
        Expr::Lit(lit) => parse_lit_segment(lit),
        Expr::Match(expr_match) => parse_match_segment(expr_match),
        other => Err(syn::Error::new_spanned(
            other,
            "classes! accepts only string literals and match expressions with string-literal arms",
        )),
    }
}

fn parse_lit_segment(lit: ExprLit) -> syn::Result<Segment> {
    let Lit::Str(value) = lit.lit else {
        return Err(syn::Error::new_spanned(
            lit,
            "classes! accepts only string literals",
        ));
    };
    Ok(Segment::Static(parse_class_literal(&value)?))
}

fn parse_match_segment(expr_match: ExprMatch) -> syn::Result<Segment> {
    let mut arms = Vec::new();
    for arm in expr_match.arms {
        let Expr::Lit(body) = *arm.body else {
            return Err(syn::Error::new_spanned(
                arm.body,
                "classes! match arms must return string literals",
            ));
        };
        let Lit::Str(value) = body.lit else {
            return Err(syn::Error::new_spanned(
                body,
                "classes! match arms must return string literals",
            ));
        };
        arms.push(MatchArm {
            pat: arm.pat,
            guard: arm.guard,
            atoms: parse_class_literal(&value)?,
        });
    }
    Ok(Segment::Match(MatchSegment {
        expr: expr_match.expr,
        arms,
    }))
}

fn parse_class_literal(value: &syn::LitStr) -> syn::Result<Vec<TokenStream>> {
    let mut atoms = Vec::new();
    for token in value.value().split(' ').filter(|token| !token.is_empty()) {
        atoms.push(parse_token(value, token)?);
    }
    Ok(atoms)
}

fn parse_token(origin: &syn::LitStr, token: &str) -> syn::Result<TokenStream> {
    reject_token(origin, token)?;
    if let Some(atom) = fixed_token(token) {
        return Ok(atom);
    }
    prefixed_token(token).ok_or_else(|| {
        syn::Error::new(
            origin.span(),
            format!("unknown class token '{token}'; help: nearest known structural tokens are p-4, px-4, w-full"),
        )
    })
}

fn reject_token(origin: &syn::LitStr, token: &str) -> syn::Result<()> {
    if token.contains("${") || token.contains('}') {
        return Err(syn::Error::new(
            origin.span(),
            "classes! does not support string interpolation",
        ));
    }
    if let Some((prefix, _rest)) = token.split_once(':') {
        return match prefix {
            "sm" | "md" | "lg" => Err(syn::Error::new(
                origin.span(),
                format!("responsive prefix '{prefix}:' is not supported"),
            )),
            "hover" | "focus" | "active" => Err(syn::Error::new(
                origin.span(),
                format!("pseudo prefix '{prefix}:' is not supported"),
            )),
            _ => Err(syn::Error::new(
                origin.span(),
                format!("unknown class prefix '{prefix}:'"),
            )),
        };
    }
    Ok(())
}

fn fixed_token(token: &str) -> Option<TokenStream> {
    match token {
        "flex" => Some(quote!(::style::StyleAtom::Display(
            ::style::DisplayKind::Flex
        ))),
        "block" => Some(quote!(::style::StyleAtom::Display(
            ::style::DisplayKind::Block
        ))),
        "hidden" => Some(quote!(::style::StyleAtom::Display(
            ::style::DisplayKind::None
        ))),
        "flex-row" => Some(quote!(::style::StyleAtom::FlexDirection(
            ::style::FlexDir::Row
        ))),
        "flex-col" => Some(quote!(::style::StyleAtom::FlexDirection(
            ::style::FlexDir::Column
        ))),
        "flex-row-reverse" => Some(quote!(::style::StyleAtom::FlexDirection(
            ::style::FlexDir::RowReverse
        ))),
        "flex-col-reverse" => Some(quote!(::style::StyleAtom::FlexDirection(
            ::style::FlexDir::ColumnReverse
        ))),
        "w-full" => Some(quote!(::style::StyleAtom::Width(::style::Dimension::Full))),
        "h-full" => Some(quote!(::style::StyleAtom::Height(::style::Dimension::Full))),
        "w-auto" => Some(quote!(::style::StyleAtom::Width(::style::Dimension::Auto))),
        "h-auto" => Some(quote!(::style::StyleAtom::Height(::style::Dimension::Auto))),
        "rounded" => Some(px_atom("BorderRadius", 4)),
        "rounded-sm" => Some(px_atom("BorderRadius", 2)),
        "rounded-md" => Some(px_atom("BorderRadius", 6)),
        "rounded-lg" => Some(px_atom("BorderRadius", 8)),
        "rounded-full" => Some(px_atom("BorderRadius", 9_999)),
        "text-xs" => Some(px_atom("FontSize", 12)),
        "text-sm" => Some(px_atom("FontSize", 14)),
        "text-base" => Some(px_atom("FontSize", 16)),
        "text-lg" => Some(px_atom("FontSize", 18)),
        "text-xl" => Some(px_atom("FontSize", 20)),
        "font-normal" => Some(quote!(::style::StyleAtom::FontWeight(400))),
        "font-medium" => Some(quote!(::style::StyleAtom::FontWeight(500))),
        "font-bold" => Some(quote!(::style::StyleAtom::FontWeight(700))),
        _ => None,
    }
}

fn prefixed_token(token: &str) -> Option<TokenStream> {
    spacing_token(token)
        .or_else(|| dimension_token(token))
        .or_else(|| align_token(token))
        .or_else(|| justify_token(token))
        .or_else(|| opacity_token(token))
        .or_else(|| theme_token(token))
}

fn spacing_token(token: &str) -> Option<TokenStream> {
    let (prefix, value) = token.split_once('-')?;
    let px = scale(value)?;
    let variant = match prefix {
        "p" => "PaddingAll",
        "px" => "PaddingX",
        "py" => "PaddingY",
        "pt" => "PaddingTop",
        "pr" => "PaddingRight",
        "pb" => "PaddingBottom",
        "pl" => "PaddingLeft",
        "m" => "MarginAll",
        "mx" => "MarginX",
        "my" => "MarginY",
        "mt" => "MarginTop",
        "mr" => "MarginRight",
        "mb" => "MarginBottom",
        "ml" => "MarginLeft",
        "gap" => "Gap",
        "leading" => "LineHeight",
        _ => return None,
    };
    Some(px_atom(variant, px))
}

fn dimension_token(token: &str) -> Option<TokenStream> {
    let (prefix, value) = token.split_once('-')?;
    let px = scale(value)?;
    match prefix {
        "w" => Some(quote!(::style::StyleAtom::Width(::style::Dimension::Px(::style::Px(#px))))),
        "h" => Some(quote!(::style::StyleAtom::Height(::style::Dimension::Px(::style::Px(#px))))),
        _ => None,
    }
}

fn align_token(token: &str) -> Option<TokenStream> {
    let value = token.strip_prefix("items-")?;
    let variant = match value {
        "start" => quote!(::style::AlignKind::Start),
        "center" => quote!(::style::AlignKind::Center),
        "end" => quote!(::style::AlignKind::End),
        "stretch" => quote!(::style::AlignKind::Stretch),
        _ => return None,
    };
    Some(quote!(::style::StyleAtom::AlignItems(#variant)))
}

fn justify_token(token: &str) -> Option<TokenStream> {
    let value = token.strip_prefix("justify-")?;
    let variant = match value {
        "start" => quote!(::style::JustifyKind::Start),
        "center" => quote!(::style::JustifyKind::Center),
        "end" => quote!(::style::JustifyKind::End),
        "between" => quote!(::style::JustifyKind::Between),
        "around" => quote!(::style::JustifyKind::Around),
        _ => return None,
    };
    Some(quote!(::style::StyleAtom::JustifyContent(#variant)))
}

fn opacity_token(token: &str) -> Option<TokenStream> {
    let value = token.strip_prefix("opacity-")?.parse::<u16>().ok()?;
    (value <= 100).then(|| {
        let encoded = value.saturating_mul(100);
        quote!(::style::StyleAtom::Opacity(#encoded))
    })
}

fn theme_token(token: &str) -> Option<TokenStream> {
    token
        .strip_prefix("bg-")
        .map(|value| quote!(::style::StyleAtom::BackgroundThemeRef(#value)))
        .or_else(|| {
            token
                .strip_prefix("text-")
                .map(|value| quote!(::style::StyleAtom::TextThemeRef(#value)))
        })
        .or_else(|| {
            token
                .strip_prefix("border-")
                .map(|_| quote!(::style::StyleAtom::UnknownThemeRef(#token)))
        })
        .or_else(|| {
            token
                .strip_prefix("shadow-")
                .map(|_| quote!(::style::StyleAtom::UnknownThemeRef(#token)))
        })
}

fn px_atom(variant: &str, px: i16) -> TokenStream {
    let variant = syn::Ident::new(variant, proc_macro2::Span::call_site());
    quote!(::style::StyleAtom::#variant(::style::Px(#px)))
}

fn scale(value: &str) -> Option<i16> {
    value.parse::<i16>().ok()?.checked_mul(4)
}
