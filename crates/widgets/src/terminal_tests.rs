use sigil_kernel::Cx;
use sigil_reactive::{Runtime, signal};

use super::terminal;

#[test]
fn terminal_builds() {
    Runtime::scope("terminal-builds", || {
        let lines = signal(vec![String::from("$ ")]);
        let mut cx = Cx::new_root();
        let id = cx.add(terminal(lines, |_| {}));
        assert!(cx.get(id).is_some());
    });
}
