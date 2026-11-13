//! The font files that Sigil bundles, as byte slices.
#![no_std]
#![deny(missing_docs)]

/// Fira Sans Regular, the sans face at weight 400.
pub const FIRA_SANS_REGULAR: &[u8] = include_bytes!("../fonts/FiraSans-Regular.ttf");
/// Fira Sans Medium, the sans face at weight 500.
pub const FIRA_SANS_MEDIUM: &[u8] = include_bytes!("../fonts/FiraSans-Medium.ttf");
/// Fira Sans at weight 600, the semibold sans face.
pub const FIRA_SANS_SEMIBOLD: &[u8] = include_bytes!("../fonts/FiraSans-SemiBold.ttf");
/// Cascadia Mono Regular, the fixed-width face.
pub const CASCADIA_MONO_REGULAR: &[u8] = include_bytes!("../fonts/CascadiaMono-Regular.ttf");
/// A subset of Noto Color Emoji.
pub const NOTO_COLOR_EMOJI_SUBSET: &[u8] = include_bytes!("../fonts/NotoColorEmoji-subset.ttf");
/// The Sigil symbol font.
pub const SIGIL_SYMBOLS: &[u8] = include_bytes!("../fonts/SigilSymbols.ttf");
/// The text of the SIL Open Font License that covers the bundled fonts.
pub const LICENSE: &[u8] = include_bytes!("../fonts/LICENSE-OFL.txt");
/// The text of the license of the `DejaVu` fonts.
pub const LICENSE_DEJAVU: &[u8] = include_bytes!("../fonts/LICENSE-DejaVu.txt");
