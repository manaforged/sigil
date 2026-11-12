use super::{
    Event, Gesture, GestureEvent, Key, KeyEvent, KeyPhase, Modifiers, MouseButton, MouseButtons,
    NamedKey, PhysicalKey, PointerEvent, PointerPhase, PointerType, ScrollDelta, ScrollEvent,
    ScrollPhase, WindowEvent,
};
use crate::geometry::Point;

impl Event {
    /// Builds a key event with no modifiers, no text, and an unidentified physical key.
    #[must_use]
    pub fn key(key: impl Into<Key>, phase: KeyPhase) -> Self {
        Self::Key(KeyEvent::new(key.into(), phase))
    }

    /// Builds a key press.
    #[must_use]
    pub fn key_down(key: impl Into<Key>) -> Self {
        Self::key(key, KeyPhase::Down)
    }

    /// Builds the press and release of a key held with `modifiers`.
    #[must_use]
    pub fn press(key: impl Into<Key>, modifiers: Modifiers) -> [Self; 2] {
        let key = key.into();
        [
            Self::key(key.clone(), KeyPhase::Down).modifiers(modifiers),
            Self::key(key, KeyPhase::Up).modifiers(modifiers),
        ]
    }

    /// Builds the key press that types `text`.
    #[must_use]
    pub fn typed(text: &str) -> Self {
        Self::key_down(Key::character(text)).text(Some(text.into()))
    }

    /// Builds the press and release of each character of `text`.
    #[must_use]
    pub fn type_text(text: &str) -> Vec<Self> {
        let mut buffer = [0_u8; 4];
        text.chars()
            .flat_map(|ch| {
                let ch: &str = ch.encode_utf8(&mut buffer);
                [Self::typed(ch), Self::key(Key::character(ch), KeyPhase::Up)]
            })
            .collect()
    }

    /// Builds a press of Tab, or of Shift+Tab when `shift` is true.
    #[must_use]
    pub fn tab(shift: bool) -> Self {
        let mut modifiers = Modifiers::empty();
        modifiers.set(Modifiers::SHIFT, shift);
        Self::key_down(NamedKey::Tab)
            .physical(PhysicalKey::Tab)
            .modifiers(modifiers)
    }

    /// Builds a mouse pointer event with no button and no modifiers.
    #[must_use]
    pub const fn pointer(phase: PointerPhase, position: Point) -> Self {
        Self::Pointer(PointerEvent::new(phase, position))
    }

    /// Builds a pointer move with no button held.
    #[must_use]
    pub const fn pointer_move(position: Point) -> Self {
        Self::pointer(PointerPhase::Move, position)
    }

    /// Builds a pointer move with the left button held.
    #[must_use]
    pub const fn pointer_drag(position: Point) -> Self {
        Self::pointer_move(position).buttons(MouseButtons::LEFT)
    }

    /// Builds a left button press.
    #[must_use]
    pub const fn pointer_down(position: Point) -> Self {
        Self::pointer(PointerPhase::Down, position)
            .button(Some(MouseButton::Left))
            .buttons(MouseButtons::LEFT)
    }

    /// Builds a left button release.
    #[must_use]
    pub const fn pointer_up(position: Point) -> Self {
        Self::pointer(PointerPhase::Up, position).button(Some(MouseButton::Left))
    }

    /// Builds the move, press, and release of a left click.
    #[must_use]
    pub const fn click(position: Point) -> [Self; 3] {
        [
            Self::pointer_move(position),
            Self::pointer_down(position),
            Self::pointer_up(position),
        ]
    }

    /// Builds a scroll event with no modifiers.
    #[must_use]
    pub const fn scroll(position: Point, delta: ScrollDelta, phase: ScrollPhase) -> Self {
        Self::Scroll(ScrollEvent::new(position, delta, phase))
    }

    /// Builds a vertical wheel move of `lines` lines.
    #[must_use]
    pub const fn wheel(position: Point, lines: f32) -> Self {
        Self::scroll(
            position,
            ScrollDelta::Lines(Point::new(0.0, lines)),
            ScrollPhase::Moved,
        )
    }

    /// Builds a touchpad gesture with no modifiers.
    #[must_use]
    pub const fn gesture(kind: Gesture, phase: ScrollPhase, position: Point) -> Self {
        Self::Gesture(GestureEvent::new(kind, phase, position))
    }

    /// Builds a paste of `text`.
    #[must_use]
    pub fn paste(text: impl Into<String>) -> Self {
        Self::Paste(text.into())
    }

    /// Builds a file drag event for `path` in `phase`.
    #[cfg(feature = "kittest")]
    #[doc(hidden)]
    #[must_use]
    pub const fn file_drop(path: Option<String>, phase: super::FileDropPhase) -> Self {
        Self::FileDrop(super::FileDropEvent { path, phase })
    }

    /// Builds the event for a window losing focus.
    #[must_use]
    pub const fn blur() -> Self {
        Self::Window(WindowEvent::Focused(false))
    }

    /// Sets the physical key of a key event; other events stay unchanged.
    #[must_use]
    pub const fn physical(mut self, physical: PhysicalKey) -> Self {
        if let Self::Key(event) = &mut self {
            event.physical = physical;
        }
        self
    }

    /// Sets the modifiers of a key, pointer, scroll, or gesture event; other events stay unchanged.
    #[must_use]
    pub const fn modifiers(mut self, modifiers: Modifiers) -> Self {
        match &mut self {
            Self::Key(event) => event.modifiers = modifiers,
            Self::Pointer(event) => event.modifiers = modifiers,
            Self::Scroll(event) => event.modifiers = modifiers,
            Self::Gesture(event) => event.modifiers = modifiers,
            _ => {}
        }
        self
    }

    /// Marks a key event as an auto-repeat; other events stay unchanged.
    #[must_use]
    pub const fn repeat(mut self, repeat: bool) -> Self {
        if let Self::Key(event) = &mut self {
            event.repeat = repeat;
        }
        self
    }

    /// Sets the text of a key event; other events stay unchanged.
    #[must_use]
    pub fn text(mut self, text: Option<String>) -> Self {
        if let Self::Key(event) = &mut self {
            event.text = text;
        }
        self
    }

    /// Sets the movement of a pointer event; other events stay unchanged.
    #[must_use]
    pub const fn delta(mut self, delta: Point) -> Self {
        if let Self::Pointer(event) = &mut self {
            event.delta = delta;
        }
        self
    }

    /// Sets the device of a pointer event; other events stay unchanged.
    #[must_use]
    pub const fn kind(mut self, kind: PointerType) -> Self {
        if let Self::Pointer(event) = &mut self {
            event.kind = kind;
        }
        self
    }

    /// Sets the button that changed in a pointer event; other events stay unchanged.
    #[must_use]
    pub const fn button(mut self, button: Option<MouseButton>) -> Self {
        if let Self::Pointer(event) = &mut self {
            event.button = button;
        }
        self
    }

    /// Sets the held buttons of a pointer event; other events stay unchanged.
    #[must_use]
    pub const fn buttons(mut self, buttons: MouseButtons) -> Self {
        if let Self::Pointer(event) = &mut self {
            event.buttons = buttons;
        }
        self
    }
}
