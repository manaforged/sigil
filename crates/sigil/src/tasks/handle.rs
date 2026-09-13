use crate::reactive::OwnerId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OwnerHandle {
    id: OwnerId,
}

impl OwnerHandle {
    #[must_use]
    pub const fn from_id(id: OwnerId) -> Self {
        Self { id }
    }

    #[must_use]
    pub fn current() -> Option<Self> {
        crate::reactive::Owner::current().map(Self::from_id)
    }

    #[must_use]
    pub const fn id(self) -> OwnerId {
        self.id
    }

    #[must_use]
    pub fn is_alive(&self) -> bool {
        crate::reactive::is_live(self.id)
    }
}

const _: fn() = || {
    const fn assert_send_sync_clone_static<T: Send + Sync + Clone + 'static>() {}
    assert_send_sync_clone_static::<OwnerHandle>();
};
