#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) enum Variant {
    #[default]
    Normal,
    Terminal,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) enum FontFamilyHint {
    #[default]
    Proportional,
    Monospace,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct VariantTokens {
    pub(crate) radius: u16,
    pub(crate) shadows: bool,
    pub(crate) hover_lift: bool,
    pub(crate) font: FontFamilyHint,
}

impl Variant {
    #[must_use]
    pub(crate) const fn tokens(self) -> VariantTokens {
        match self {
            Self::Normal => VariantTokens {
                radius: 8,
                shadows: true,
                hover_lift: true,
                font: FontFamilyHint::Proportional,
            },
            Self::Terminal => VariantTokens {
                radius: 0,
                shadows: false,
                hover_lift: false,
                font: FontFamilyHint::Monospace,
            },
        }
    }
}

#[cfg(test)]
#[path = "variant_tests.rs"]
mod tests;
