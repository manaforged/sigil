use crate::events::{Event, PointerPhase, WindowEvent};
use crate::profile::Cause;
use crate::profile::interact;

kittest_pub! {
    /// Marks the current frame with the cause that matches an input event.
    #[doc(hidden)]
    pub fn note_input(event: &Event) {
        if !super::enabled() {
            return;
        }
        match event {
            Event::Pointer(pointer) => match pointer.phase {
                PointerPhase::Down | PointerPhase::Up => {
                    interact(Cause::Pointer);
                }
                PointerPhase::Move => interact(Cause::Hover),
                PointerPhase::Enter | PointerPhase::Leave | PointerPhase::Cancel => {}
            },
            Event::Key(_) | Event::Paste(_) => interact(Cause::Key),
            Event::Scroll(_) | Event::Gesture(_) => interact(Cause::Scroll),
            Event::Ime(_) => interact(Cause::Ime),
            Event::Window(WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged(_)) => {
                interact(Cause::Resize);
            }
            Event::Window(_) | Event::FileDrop(_) => {}
        }
    }
}
