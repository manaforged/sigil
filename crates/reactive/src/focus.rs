use core::marker::PhantomData;
use core::sync::atomic::{AtomicU64, Ordering};

use sigil_actions::HandlerKey;

static FOCUS_HANDLE_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FocusHandleId(pub(crate) u64);

impl FocusHandleId {
    #[must_use]
    pub const fn as_raw(self) -> u64 {
        self.0
    }
}

impl From<FocusHandleId> for HandlerKey {
    fn from(id: FocusHandleId) -> Self {
        Self::from_raw(id.0)
    }
}

#[derive(Debug)]
pub struct FocusHandle {
    pub(crate) id: FocusHandleId,
    _not_send: PhantomData<*const ()>,
}

impl FocusHandle {
    pub(crate) fn new() -> Self {
        let raw = FOCUS_HANDLE_COUNTER.fetch_add(1, Ordering::Relaxed);
        Self {
            id: FocusHandleId(raw),
            _not_send: PhantomData,
        }
    }

    #[must_use]
    pub const fn id(&self) -> FocusHandleId {
        self.id
    }
}
