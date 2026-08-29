#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Variant {
    #[default]
    Normal,
    Terminal,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FontFamilyHint {
    #[default]
    Proportional,
    Monospace,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VariantTokens {
    pub radius: u16,
    pub shadows: bool,
    pub hover_lift: bool,
    pub font: FontFamilyHint,
}

impl Variant {
    #[must_use]
    pub const fn tokens(self) -> VariantTokens {
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
