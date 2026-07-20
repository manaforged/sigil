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
        if let Some(expanded) = overflow_tokens(value, token)? {
            atoms.extend(expanded);
            continue;
        }
        atoms.push(parse_token(value, token)?);
    }
    Ok(atoms)
}

/// `overflow-*` expands to both axes, `overflow-x-*`/`overflow-y-*` to
/// one. Mirrors the runtime parser's `push_overflow_atoms`.
fn overflow_tokens(origin: &syn::LitStr, token: &str) -> syn::Result<Option<Vec<TokenStream>>> {
    let Some(rest) = token.strip_prefix("overflow-") else {
        return Ok(None);
    };
    let unknown = || {
        syn::Error::new(
            origin.span(),
            format!("unknown overflow token '{token}'; expected visible, hidden, or scroll"),
        )
    };
    if let Some(value) = rest.strip_prefix("x-") {
        let kind = overflow_kind(value).ok_or_else(unknown)?;
        return Ok(Some(vec![
            quote!(::sigil::__private::style::StyleAtom::OverflowX(#kind)),
        ]));
    }
    if let Some(value) = rest.strip_prefix("y-") {
        let kind = overflow_kind(value).ok_or_else(unknown)?;
        return Ok(Some(vec![
            quote!(::sigil::__private::style::StyleAtom::OverflowY(#kind)),
        ]));
    }
    let kind = overflow_kind(rest).ok_or_else(unknown)?;
    Ok(Some(vec![
        quote!(::sigil::__private::style::StyleAtom::OverflowX(#kind)),
        quote!(::sigil::__private::style::StyleAtom::OverflowY(#kind)),
    ]))
}

fn overflow_kind(value: &str) -> Option<TokenStream> {
    match value {
        "visible" => Some(quote!(::sigil::__private::style::OverflowKind::Visible)),
        "hidden" => Some(quote!(::sigil::__private::style::OverflowKind::Hidden)),
        "scroll" => Some(quote!(::sigil::__private::style::OverflowKind::Scroll)),
        "auto" => Some(quote!(::sigil::__private::style::OverflowKind::Auto)),
        _ => None,
    }
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
    if token.starts_with("shadow-") {
        return Err(syn::Error::new(
            origin.span(),
            format!(
                "'{token}' has no paint path yet; shadow-* tokens land with the \
                 renderer shadow atoms (DR-056)"
            ),
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
    fixed_layout_token(token).or_else(|| fixed_typo_token(token))
}

fn fixed_layout_token(token: &str) -> Option<TokenStream> {
    match token {
        "flex" => Some(quote!(::sigil::__private::style::StyleAtom::Display(
            ::sigil::__private::style::DisplayKind::Flex
        ))),
        "block" => Some(quote!(::sigil::__private::style::StyleAtom::Display(
            ::sigil::__private::style::DisplayKind::Block
        ))),
        "hidden" => Some(quote!(::sigil::__private::style::StyleAtom::Display(
            ::sigil::__private::style::DisplayKind::None
        ))),
        "flex-row" => Some(quote!(::sigil::__private::style::StyleAtom::FlexDirection(
            ::sigil::__private::style::FlexDir::Row
        ))),
        "flex-col" => Some(quote!(::sigil::__private::style::StyleAtom::FlexDirection(
            ::sigil::__private::style::FlexDir::Column
        ))),
        "flex-row-reverse" => Some(quote!(::sigil::__private::style::StyleAtom::FlexDirection(
            ::sigil::__private::style::FlexDir::RowReverse
        ))),
        "flex-col-reverse" => Some(quote!(::sigil::__private::style::StyleAtom::FlexDirection(
            ::sigil::__private::style::FlexDir::ColumnReverse
        ))),
        "flex-wrap" => Some(quote!(::sigil::__private::style::StyleAtom::FlexWrap(
            ::sigil::__private::style::FlexWrapKind::Wrap
        ))),
        "flex-nowrap" => Some(quote!(::sigil::__private::style::StyleAtom::FlexWrap(
            ::sigil::__private::style::FlexWrapKind::NoWrap
        ))),
        "w-full" => Some(quote!(::sigil::__private::style::StyleAtom::Width(
            ::sigil::__private::style::Dimension::Full
        ))),
        "h-full" => Some(quote!(::sigil::__private::style::StyleAtom::Height(
            ::sigil::__private::style::Dimension::Full
        ))),
        "w-auto" => Some(quote!(::sigil::__private::style::StyleAtom::Width(
            ::sigil::__private::style::Dimension::Auto
        ))),
        "h-auto" => Some(quote!(::sigil::__private::style::StyleAtom::Height(
            ::sigil::__private::style::Dimension::Auto
        ))),
        _ => None,
    }
}

fn fixed_typo_token(token: &str) -> Option<TokenStream> {
    match token {
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
        "font-normal" => Some(quote!(::sigil::__private::style::StyleAtom::FontWeight(
            400
        ))),
        "font-medium" => Some(quote!(::sigil::__private::style::StyleAtom::FontWeight(
            500
        ))),
        "font-bold" => Some(quote!(::sigil::__private::style::StyleAtom::FontWeight(
            700
        ))),
        "font-sans" => Some(quote!(::sigil::__private::style::StyleAtom::FontFamily(
            "sans"
        ))),
        "font-mono" => Some(quote!(::sigil::__private::style::StyleAtom::FontFamily(
            "mono"
        ))),
        "font-serif" => Some(quote!(::sigil::__private::style::StyleAtom::FontFamily(
            "serif"
        ))),
        "font-icon" => Some(quote!(::sigil::__private::style::StyleAtom::FontFamily(
            "icon"
        ))),
        "grow" => Some(quote!(::sigil::__private::style::StyleAtom::FlexGrow(
            1000u16
        ))),
        "grow-0" => Some(quote!(::sigil::__private::style::StyleAtom::FlexGrow(0u16))),
        "shrink" => Some(quote!(::sigil::__private::style::StyleAtom::FlexShrink(
            1000u16
        ))),
        "shrink-0" => Some(quote!(::sigil::__private::style::StyleAtom::FlexShrink(
            0u16
        ))),
        "border" => Some(px_atom("BorderWidth", 1)),
        _ => None,
    }
}

fn prefixed_token(token: &str) -> Option<TokenStream> {
    spacing_token(token)
        .or_else(|| dimension_token(token))
        .or_else(|| align_token(token))
        .or_else(|| justify_token(token))
        .or_else(|| opacity_token(token))
        .or_else(|| border_token(token))
        .or_else(|| theme_token(token))
}

/// `border-N` is a literal pixel width (Tailwind shape); `border-<token>`
/// is a theme colour reference. Mirrors the runtime parser's `border_atom`.
fn border_token(token: &str) -> Option<TokenStream> {
    let value = token.strip_prefix("border-")?;
    if let Ok(width) = value.parse::<i16>() {
        return (width >= 0).then(|| px_atom("BorderWidth", width));
    }
    Some(quote!(::sigil::__private::style::StyleAtom::BorderThemeRef(#value)))
}

fn spacing_token(token: &str) -> Option<TokenStream> {
    if let Some(value) = token.strip_prefix("gap-x-") {
        let px = scale(value)?;
        return Some(px_atom("GapX", px));
    }
    if let Some(value) = token.strip_prefix("gap-y-") {
        let px = scale(value)?;
        return Some(px_atom("GapY", px));
    }
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
    for (prefix, variant) in [
        ("min-w-", "MinWidth"),
        ("max-w-", "MaxWidth"),
        ("min-h-", "MinHeight"),
        ("max-h-", "MaxHeight"),
    ] {
        if let Some(value) = token.strip_prefix(prefix) {
            let px = scale(value)?;
            let variant = syn::Ident::new(variant, proc_macro2::Span::call_site());
            return Some(quote!(::sigil::__private::style::StyleAtom::#variant(
                ::sigil::__private::style::Dimension::Px(::sigil::__private::style::Px(#px))
            )));
        }
    }
    let (prefix, value) = token.split_once('-')?;
    let px = scale(value)?;
    match prefix {
        "w" => Some(
            quote!(::sigil::__private::style::StyleAtom::Width(::sigil::__private::style::Dimension::Px(::sigil::__private::style::Px(#px)))),
        ),
        "h" => Some(
            quote!(::sigil::__private::style::StyleAtom::Height(::sigil::__private::style::Dimension::Px(::sigil::__private::style::Px(#px)))),
        ),
        _ => None,
    }
}

fn align_token(token: &str) -> Option<TokenStream> {
    let value = token.strip_prefix("items-")?;
    let variant = match value {
        "start" => quote!(::sigil::__private::style::AlignKind::Start),
        "center" => quote!(::sigil::__private::style::AlignKind::Center),
        "end" => quote!(::sigil::__private::style::AlignKind::End),
        "stretch" => quote!(::sigil::__private::style::AlignKind::Stretch),
        _ => return None,
    };
    Some(quote!(::sigil::__private::style::StyleAtom::AlignItems(#variant)))
}

fn justify_token(token: &str) -> Option<TokenStream> {
    let value = token.strip_prefix("justify-")?;
    let variant = match value {
        "start" => quote!(::sigil::__private::style::JustifyKind::Start),
        "center" => quote!(::sigil::__private::style::JustifyKind::Center),
        "end" => quote!(::sigil::__private::style::JustifyKind::End),
        "between" => quote!(::sigil::__private::style::JustifyKind::Between),
        "around" => quote!(::sigil::__private::style::JustifyKind::Around),
        "evenly" => quote!(::sigil::__private::style::JustifyKind::Evenly),
        _ => return None,
    };
    Some(quote!(::sigil::__private::style::StyleAtom::JustifyContent(#variant)))
}

fn opacity_token(token: &str) -> Option<TokenStream> {
    let value = token.strip_prefix("opacity-")?.parse::<u16>().ok()?;
    (value <= 100).then(|| {
        let encoded = value.saturating_mul(100);
        quote!(::sigil::__private::style::StyleAtom::Opacity(#encoded))
    })
}

fn theme_token(token: &str) -> Option<TokenStream> {
    token
        .strip_prefix("bg-")
        .map(|value| quote!(::sigil::__private::style::StyleAtom::BackgroundThemeRef(#value)))
        .or_else(|| {
            token
                .strip_prefix("text-")
                .map(|value| quote!(::sigil::__private::style::StyleAtom::TextThemeRef(#value)))
        })
}

fn px_atom(variant: &str, px: i16) -> TokenStream {
    let variant = syn::Ident::new(variant, proc_macro2::Span::call_site());
    quote!(::sigil::__private::style::StyleAtom::#variant(::sigil::__private::style::Px(#px)))
}

fn scale(value: &str) -> Option<i16> {
    value.parse::<i16>().ok()?.checked_mul(4)
}
