use crate::kernel::Cx;
use crate::reactive::{Runtime, signal};

use super::terminal;

#[test]
fn terminal_builds() {
    Runtime::scope("terminal-builds", || {
        let lines = signal(vec![String::from("$ ")]);
        let mut cx = Cx::new_root();
        let id = cx.add(terminal(lines, |_| {}));
        let node = cx.get(id).expect("terminal root");
        assert!(!node.children().is_empty(), "the terminal shows its lines");
    });
}
