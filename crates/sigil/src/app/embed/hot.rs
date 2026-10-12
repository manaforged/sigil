use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Once, Weak};

use crate::events::EventRouter;
use crate::kernel::{AccessibleName, Cx, IntoNode, Node, NodeId};
use crate::reactive::{OwnerId, Runtime};
use crate::tasks::OwnerHandle;
use crate::tasks::dispatch::dispatch_to_owner;
use crate::tasks::runtime::Dispatcher;

use super::Host;

type Build = Box<dyn FnMut(&mut Cx) -> NodeId>;

type PatchHandler = Arc<dyn Fn() + Send + Sync + 'static>;

pub(crate) struct HotRoot {
    build: Build,
    patched: Arc<AtomicBool>,
    #[cfg(test)]
    handler: PatchHandler,
}

impl HotRoot {
    fn new<F, N>(root: F, owner: OwnerId) -> Self
    where
        F: Fn() -> N + 'static,
        N: IntoNode,
    {
        let patched = Arc::new(AtomicBool::new(false));
        let dispatcher = crate::tasks::current::dispatcher();
        let handler = on_patch(Arc::downgrade(&patched), dispatcher, owner);
        subsecond::register_handler(Arc::clone(&handler));
        Self {
            build: Box::new(move |cx| {
                subsecond::HotFn::current(call_root::<F, N>).call((&root, cx))
            }),
            patched,
            #[cfg(test)]
            handler,
        }
    }

    fn install(&mut self, cx: &mut Cx) -> NodeId {
        (self.build)(cx)
    }

    fn take_patched(&self) -> bool {
        self.patched.swap(false, Ordering::AcqRel)
    }
}

fn call_root<F: Fn() -> N, N: IntoNode>(root: &F, cx: &mut Cx) -> NodeId {
    cx.add(root)
}

fn on_patch(
    patched: Weak<AtomicBool>,
    dispatcher: Option<Dispatcher>,
    owner: OwnerId,
) -> PatchHandler {
    Arc::new(move || {
        let Some(flag) = patched.upgrade() else {
            return;
        };
        flag.store(true, Ordering::Release);
        if let Some(dispatcher) = &dispatcher {
            dispatch_to_owner(
                dispatcher,
                OwnerHandle::from_id(owner),
                Runtime::mark_frame_dirty,
            );
        }
    })
}

impl Host {
    pub fn mount_hot<F, N>(&mut self, root: F)
    where
        F: Fn() -> N + 'static,
        N: IntoNode,
    {
        static CONNECT: Once = Once::new();
        CONNECT.call_once(dioxus_devtools::connect_subsecond);
        let mut hot = HotRoot::new(root, self.cx.root_owner());
        self.replace_root(|cx| hot.install(cx));
        self.hot = Some(hot);
    }

    #[cfg(test)]
    #[allow(dead_code, reason = "the seam that fires a patch without a devserver")]
    pub(crate) fn fire_hot_patch(&self) -> bool {
        self.hot.as_ref().map(|hot| (hot.handler)()).is_some()
    }

    pub(crate) fn remount_patched(&mut self, router: &EventRouter) {
        let Some(mut hot) = self.hot.take_if(|hot| hot.take_patched()) else {
            return;
        };
        let focus = router
            .focus()
            .zip(self.root)
            .and_then(|(id, root)| focus_path(&self.cx, root, id));
        self.replace_root(|cx| hot.install(cx));
        self.hot = Some(hot);
        if let Some(id) =
            focus.and_then(|path| self.root.and_then(|root| find(&self.cx, root, &path)))
        {
            crate::events::focus::request_focus(id);
        }
    }
}

#[derive(PartialEq, Eq)]
struct Step {
    role: accesskit::Role,
    name: String,
    ordinal: usize,
}

fn name(node: &Node) -> String {
    match &node.a11y.name {
        AccessibleName::None => String::new(),
        AccessibleName::Static(text) => text.clone(),
        AccessibleName::Derived(signal) => signal.peek(),
    }
}

fn same(a: &Node, b: &Node) -> bool {
    a.a11y.role == b.a11y.role && name(a) == name(b)
}

fn focus_path(cx: &Cx, root: NodeId, focused: NodeId) -> Option<Vec<Step>> {
    let mut path = Vec::new();
    let mut id = focused;
    while id != root {
        let node = cx.get(id)?;
        let parent = node.parent()?;
        let siblings = cx.get(parent)?.children();
        let ordinal = siblings
            .iter()
            .take_while(|&&sibling| sibling != id)
            .filter(|&&sibling| cx.get(sibling).is_some_and(|other| same(other, node)))
            .count();
        path.push(Step {
            role: node.a11y.role,
            name: name(node),
            ordinal,
        });
        id = parent;
    }
    path.reverse();
    Some(path)
}

fn find(cx: &Cx, root: NodeId, path: &[Step]) -> Option<NodeId> {
    path.iter().try_fold(root, |parent, step| {
        cx.get(parent)?
            .children()
            .iter()
            .copied()
            .filter(|&child| {
                cx.get(child)
                    .is_some_and(|node| node.a11y.role == step.role && name(node) == step.name)
            })
            .nth(step.ordinal)
    })
}

#[cfg(all(test, feature = "terminal"))]
#[path = "hot_tests.rs"]
mod tests;
