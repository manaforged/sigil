//! Internal input-event push helpers on [`super::WinitShell`].
//!
//! Split out of `shell.rs` to keep the lifecycle / Shell-impl portions
//! readable under the 400-LoC hygiene cap. These methods translate raw
//! winit events into the sigil-hal `ShellEvent` vocabulary.

use hal::{
    ImeEvent, KeyEvent as SigilKeyEvent, KeyEventInit, KeyPhase, MouseButton, PhysicalSize, Point,
    PointerEvent, PointerEventInit, PointerKind, PointerPhase, Scale, ScrollEvent, ScrollEventInit,
    ShellEvent, WindowEvent as SigilWindowEvent,
};
use smol_str::SmolStr;
use winit::{
    event::{
        ElementState, Ime as WinitIme, KeyEvent as WinitKeyEvent, MouseButton as WinitMouseButton,
        MouseScrollDelta as WinitMouseScrollDelta, TouchPhase,
    },
    window::WindowId,
};

use crate::dpi::{logical_point_from_f64, scale_from_f64, set_scale_for_window_id};
use crate::event_map::{
    map_modifiers, map_mouse_button, map_scroll_delta, map_scroll_phase, map_touch_pointer_phase,
    mouse_button_bit,
};
use crate::key_map::{map_logical_key, map_physical_key};

use super::WinitShell;

impl WinitShell {
    pub(super) fn push_window_resized(&mut self, size: winit::dpi::PhysicalSize<u32>) -> bool {
        self.push_window(SigilWindowEvent::Resized(PhysicalSize::new(
            size.width,
            size.height,
        )))
    }

    pub(super) fn push_window(&mut self, event: SigilWindowEvent) -> bool {
        self.events.push(ShellEvent::Window(event));
        true
    }

    pub(super) fn push_scale_factor_changed(
        &mut self,
        window_id: WindowId,
        scale_factor: f64,
    ) -> bool {
        let scale = scale_from_f64(scale_factor);
        set_scale_for_window_id(&self.windows, &mut self.window_scales, window_id, scale);
        self.push_window(SigilWindowEvent::ScaleFactorChanged(scale))
    }

    pub(super) fn push_focus(&mut self, focused: bool) -> bool {
        self.push_window(SigilWindowEvent::Focused(focused));
        if !focused {
            self.push_pointer_phase(PointerPhase::Leave);
        }
        true
    }

    pub(super) fn update_modifiers(&mut self, modifiers: winit::event::Modifiers) -> bool {
        self.modifiers = map_modifiers(modifiers.state());
        false
    }

    pub(super) fn push_key(&mut self, event: WinitKeyEvent) -> bool {
        let phase = match event.state {
            ElementState::Pressed => KeyPhase::Down,
            ElementState::Released => KeyPhase::Up,
        };
        let text = event.text.as_ref().map(|text| SmolStr::new(text.as_str()));
        self.events
            .push(ShellEvent::Key(SigilKeyEvent::new(KeyEventInit {
                key: map_logical_key(event.logical_key),
                physical: map_physical_key(event.physical_key),
                modifiers: self.modifiers,
                repeat: event.repeat,
                phase,
                text,
            })));
        true
    }

    pub(super) fn push_ime(&mut self, event: WinitIme) -> bool {
        let event = match event {
            WinitIme::Enabled => ImeEvent::Enabled,
            WinitIme::Preedit(text, range) => ImeEvent::Preedit(SmolStr::new(&text), range),
            WinitIme::Commit(text) => ImeEvent::Commit(SmolStr::new(&text)),
            WinitIme::Disabled => ImeEvent::Disabled,
        };
        self.events.push(ShellEvent::Ime(event));
        true
    }

    pub(super) fn push_cursor_move(
        &mut self,
        position: winit::dpi::PhysicalPosition<f64>,
        scale: Scale,
    ) -> bool {
        let next = logical_point_from_f64(position.x, position.y, scale);
        let delta = Point::new(
            next.x - self.pointer_position.x,
            next.y - self.pointer_position.y,
        );
        self.pointer_position = next;
        self.push_pointer(PointerPhase::Move, delta, None, PointerKind::Mouse);
        true
    }

    pub(super) fn push_pointer_phase(&mut self, phase: PointerPhase) -> bool {
        self.push_pointer(phase, Point::new(0.0, 0.0), None, PointerKind::Mouse);
        true
    }

    pub(super) fn push_mouse_input(&mut self, state: ElementState, button: WinitMouseButton) {
        let button = map_mouse_button(button);
        if let Some(bit) = mouse_button_bit(button) {
            match state {
                ElementState::Pressed => self.pointer_buttons.insert(bit),
                ElementState::Released => self.pointer_buttons.remove(bit),
            }
        }
        let phase = match state {
            ElementState::Pressed => PointerPhase::Down,
            ElementState::Released => PointerPhase::Up,
        };
        self.push_pointer(
            phase,
            Point::new(0.0, 0.0),
            Some(button),
            PointerKind::Mouse,
        );
    }

    pub(super) fn push_touch(&mut self, phase: TouchPhase, x: f64, y: f64, scale: Scale) {
        let next = logical_point_from_f64(x, y, scale);
        let delta = Point::new(
            next.x - self.pointer_position.x,
            next.y - self.pointer_position.y,
        );
        self.pointer_position = next;
        self.push_pointer(
            map_touch_pointer_phase(phase),
            delta,
            None,
            PointerKind::Touch,
        );
    }

    fn push_pointer(
        &mut self,
        phase: PointerPhase,
        delta: Point,
        button: Option<MouseButton>,
        kind: PointerKind,
    ) {
        self.events
            .push(ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
                position: self.pointer_position,
                delta,
                kind,
                button,
                buttons: self.pointer_buttons,
                modifiers: self.modifiers,
                phase,
            })));
    }

    pub(super) fn push_scroll(
        &mut self,
        delta: WinitMouseScrollDelta,
        phase: TouchPhase,
        scale: Scale,
    ) -> bool {
        self.events
            .push(ShellEvent::Scroll(ScrollEvent::new(ScrollEventInit {
                position: self.pointer_position,
                delta: map_scroll_delta(delta, scale),
                modifiers: self.modifiers,
                phase: map_scroll_phase(phase),
            })));
        true
    }
}
