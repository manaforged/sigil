#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::panic, clippy::unwrap_used, clippy::indexing_slicing)]

use std::cell::Cell;
use std::rc::Rc;

use sigil::kernel::{A11yProps, Cx, IntoNode, NodeId, NodeType, container};
use sigil::reactive::{Runtime, debug, effect, on_cleanup, signal};

struct DisposableComponent {
    cleanup_count: Rc<Cell<u32>>,
    effect_runs: Rc<Cell<u32>>,
}

impl IntoNode for DisposableComponent {
    fn build(self, cx: &mut Cx) -> NodeId {
        let a = signal(1_i32);
        let b = signal(2_i32);
        let c = signal(3_i32);

        let cleanup_count = Rc::clone(&self.cleanup_count);
        on_cleanup(move || cleanup_count.set(cleanup_count.get() + 1));

        let effect_runs = Rc::clone(&self.effect_runs);
        effect(move || {
            let _ = a.get() + b.get() + c.get();
            effect_runs.set(effect_runs.get() + 1);
        });

        cx.new_node(NodeType::Container, A11yProps::default())
    }
}

#[test]
fn remove_subtree_disposes_reactive_owner_resources() {
    let cleanup_count = Rc::new(Cell::new(0_u32));
    let effect_runs = Rc::new(Cell::new(0_u32));

    Runtime::scope("sigil_kernel_owner_disposal", || {
        let mut cx = Cx::new_root();
        let root = cx.add(container().child(DisposableComponent {
            cleanup_count: Rc::clone(&cleanup_count),
            effect_runs: Rc::clone(&effect_runs),
        }));

        assert_eq!(effect_runs.get(), 1, "effect ran on registration");
        assert_eq!(cx.len(), 2);

        let before = debug::dump_owner_tree();
        assert!(before.contains("signals: 3"));
        assert!(before.contains("effects: 1"));

        let child = cx.get(root).expect("root").children()[0];
        assert!(cx.remove(child));

        assert_eq!(cleanup_count.get(), 1, "component cleanup fired once");
        assert_eq!(cx.len(), 1);
        assert!(cx.get(child).is_none());
        assert!(
            cx.get(root)
                .expect("root still alive")
                .children()
                .is_empty()
        );

        let after = debug::dump_owner_tree();
        assert!(!after.contains("signals: 3"));
        assert!(!after.contains("effects: 1"));
    });
}
