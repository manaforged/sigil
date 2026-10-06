#[cfg(feature = "scene")]
use crate::icons::Icon;
use crate::kernel::TextContent;

use super::{ButtonBuilder, button};

pub enum ButtonGlyph {
    Text(TextContent),
    #[cfg(feature = "scene")]
    Icon(Icon),
}

impl<T: Into<TextContent>> From<T> for ButtonGlyph {
    fn from(text: T) -> Self {
        Self::Text(text.into())
    }
}

#[cfg(feature = "scene")]
impl From<Icon> for ButtonGlyph {
    fn from(icon: Icon) -> Self {
        Self::Icon(icon)
    }
}

impl ButtonGlyph {
    pub(super) fn into_button(self) -> ButtonBuilder {
        match self {
            Self::Text(text) => button(text),
            #[cfg(feature = "scene")]
            Self::Icon(icon) => button(icon.name()).icon(icon).accessible(icon.name()),
        }
    }
}
