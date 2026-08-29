use crate::{StateStyle, StyleAtom, StyleVariant};

static SOLID_BASE: &[StyleAtom] = &[StyleAtom::BgToneStep(9), StyleAtom::FgOnSolid];
static SOLID_REST: &[StyleAtom] = &[];
static SOLID_FOCUS: &[StyleAtom] = &[StyleAtom::BgToneStep(10)];
static SOLID_HOVER: &[StyleAtom] = &[StyleAtom::BgToneStep(10)];
static SOLID_PRESSED: &[StyleAtom] = &[StyleAtom::BgToneStep(11)];
static SOLID_DISABLED: &[StyleAtom] = &[StyleAtom::BgToneStep(8), StyleAtom::Opacity(5_000)];

const SOLID_STATES: StateStyle = StateStyle::new(
    SOLID_REST,
    SOLID_FOCUS,
    SOLID_HOVER,
    SOLID_PRESSED,
    SOLID_DISABLED,
);

pub const SOLID_VARIANT: StyleVariant = StyleVariant::new(SOLID_BASE, SOLID_STATES);

static SOFT_BASE: &[StyleAtom] = &[StyleAtom::BgToneStep(4), StyleAtom::FgToneStep(11)];
static SOFT_REST: &[StyleAtom] = &[];
static SOFT_FOCUS: &[StyleAtom] = &[StyleAtom::BgToneStep(5)];
static SOFT_HOVER: &[StyleAtom] = &[StyleAtom::BgToneStep(5)];
static SOFT_PRESSED: &[StyleAtom] = &[StyleAtom::BgToneStep(6)];
static SOFT_DISABLED: &[StyleAtom] = &[StyleAtom::BgToneStep(2), StyleAtom::Opacity(5_000)];

const SOFT_STATES: StateStyle = StateStyle::new(
    SOFT_REST,
    SOFT_FOCUS,
    SOFT_HOVER,
    SOFT_PRESSED,
    SOFT_DISABLED,
);

pub const SOFT_VARIANT: StyleVariant = StyleVariant::new(SOFT_BASE, SOFT_STATES);

static GHOST_BASE: &[StyleAtom] = &[StyleAtom::FgToneStep(11)];
static GHOST_REST: &[StyleAtom] = &[];
static GHOST_FOCUS: &[StyleAtom] = &[StyleAtom::BgToneStep(4)];
static GHOST_HOVER: &[StyleAtom] = &[StyleAtom::BgToneStep(4)];
static GHOST_PRESSED: &[StyleAtom] = &[StyleAtom::BgToneStep(5)];
static GHOST_DISABLED: &[StyleAtom] = &[StyleAtom::Opacity(5_000)];

const GHOST_STATES: StateStyle = StateStyle::new(
    GHOST_REST,
    GHOST_FOCUS,
    GHOST_HOVER,
    GHOST_PRESSED,
    GHOST_DISABLED,
);

pub const GHOST_VARIANT: StyleVariant = StyleVariant::new(GHOST_BASE, GHOST_STATES);
