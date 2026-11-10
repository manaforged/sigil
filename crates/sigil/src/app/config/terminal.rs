use super::{OtherKeys, TerminalHooks};

#[derive(Debug, Clone)]
pub(crate) struct TerminalOptions {
    pub(crate) hooks: TerminalHooks,
    pub(crate) access: crate::terminal::Access,
    pub(crate) mouse_tracking: Option<crate::terminal::MouseTracking>,
    pub(crate) other_keys: OtherKeys,
    pub(crate) suspend: Option<crate::events::Shortcut>,
}

impl Default for TerminalOptions {
    fn default() -> Self {
        Self {
            hooks: TerminalHooks::default(),
            access: crate::terminal::Access::default(),
            mouse_tracking: None,
            other_keys: OtherKeys::Plain,
            suspend: Some(crate::events::Shortcut::ctrl(
                crate::events::LogicalKey::character("z"),
            )),
        }
    }
}
