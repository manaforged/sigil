use crate::events::{PointerPhase, ShellEvent, WindowEvent};
use crate::profile::Cause;
use crate::profile::interact;

pub(crate) fn note_input(event: &ShellEvent) {
    if !super::enabled() {
        return;
    }
    match event {
        ShellEvent::Pointer(pointer) => match pointer.phase {
            PointerPhase::Down | PointerPhase::Up => {
                interact(Cause::Pointer);
            }
            PointerPhase::Move => interact(Cause::Hover),
            PointerPhase::Enter | PointerPhase::Leave | PointerPhase::Cancel => {}
        },
        ShellEvent::Key(_) => interact(Cause::Key),
        ShellEvent::Scroll(_) => interact(Cause::Scroll),
        ShellEvent::Ime(_) => interact(Cause::Ime),
        ShellEvent::Window(WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged(_)) => {
            interact(Cause::Resize);
        }
        ShellEvent::Window(_) | ShellEvent::FileDrop(_) => {}
    }
}
