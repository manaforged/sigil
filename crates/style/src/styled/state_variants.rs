use super::{StateHost, StateStyleBuilder, StyleAccess};

pub trait StateVariants: StyleAccess + StateHost {
    #[must_use]
    fn rest<F>(mut self, configure: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
    {
        let built = configure(StateStyleBuilder::new());
        self.style_mut().set_rest(built.into_atoms());
        self
    }

    #[must_use]
    fn hover<F>(mut self, configure: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
    {
        let built = configure(StateStyleBuilder::new());
        self.style_mut().set_hover(built.into_atoms());
        self
    }

    #[must_use]
    fn focus<F>(mut self, configure: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
    {
        let built = configure(StateStyleBuilder::new());
        self.style_mut().set_focus(built.into_atoms());
        self
    }

    #[must_use]
    fn pressed<F>(mut self, configure: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
    {
        let built = configure(StateStyleBuilder::new());
        self.style_mut().set_pressed(built.into_atoms());
        self
    }

    #[must_use]
    fn disabled_style<F>(mut self, configure: F) -> Self
    where
        F: FnOnce(StateStyleBuilder) -> StateStyleBuilder,
    {
        let built = configure(StateStyleBuilder::new());
        self.style_mut().set_disabled(built.into_atoms());
        self
    }
}

impl<T: StyleAccess + StateHost> StateVariants for T {}
