use std::cell::RefCell;
use std::collections::HashMap;

use crate::kernel::NodeBuilder;

type NodeHook = Box<dyn FnOnce(NodeBuilder) -> NodeBuilder>;

struct HookSlot {
    refs: usize,
    hooks: Vec<NodeHook>,
}

struct HookTable {
    next: u64,
    slots: HashMap<u64, HookSlot>,
}

thread_local! {
    static HOOKS: RefCell<HookTable> = RefCell::new(HookTable {
        next: 1,
        slots: HashMap::new(),
    });
}

#[derive(Debug, Default)]
pub(crate) struct NodeHooks {
    id: Option<u64>,
}

impl Clone for NodeHooks {
    fn clone(&self) -> Self {
        if let Some(id) = self.id {
            retain(id);
        }
        Self { id: self.id }
    }
}

impl Drop for NodeHooks {
    fn drop(&mut self) {
        if let Some(id) = self.id {
            release(id);
        }
    }
}

impl NodeHooks {
    pub(crate) const fn new() -> Self {
        Self { id: None }
    }

    pub(crate) fn push(&mut self, hook: NodeHook) {
        let id = *self.id.get_or_insert_with(alloc_slot);
        HOOKS.with(|table| {
            table
                .borrow_mut()
                .slots
                .get_mut(&id)
                .expect("hook slot")
                .hooks
                .push(hook);
        });
    }

    pub(crate) fn finish(&self, node: NodeBuilder) -> NodeBuilder {
        let Some(id) = self.id else {
            return node;
        };
        let hooks = HOOKS.with(|table| {
            table
                .borrow_mut()
                .slots
                .get_mut(&id)
                .map(|slot| std::mem::take(&mut slot.hooks))
                .unwrap_or_default()
        });
        hooks.into_iter().fold(node, |built, hook| hook(built))
    }
}

fn alloc_slot() -> u64 {
    HOOKS.with(|table| {
        let mut table = table.borrow_mut();
        let id = table.next;
        table.next += 1;
        table.slots.insert(
            id,
            HookSlot {
                refs: 1,
                hooks: Vec::new(),
            },
        );
        id
    })
}

fn retain(id: u64) {
    let _ = HOOKS.try_with(|table| {
        if let Some(slot) = table.borrow_mut().slots.get_mut(&id) {
            slot.refs += 1;
        }
    });
}

fn release(id: u64) {
    let _ = HOOKS.try_with(|table| {
        let mut table = table.borrow_mut();
        let Some(slot) = table.slots.get_mut(&id) else {
            return;
        };
        slot.refs -= 1;
        if slot.refs == 0 {
            table.slots.remove(&id);
        }
    });
}
