use super::*;

impl Host {
    /// Returns the accessibility tree.
    #[must_use]
    pub fn accessibility_tree(&self) -> AccessibilityTree {
        self.accessibility_tree_scoped(crate::a11y::mapping::TreeScope::Full)
    }

    /// Renders the accessibility tree as indented text.
    #[must_use]
    pub fn accessibility_text(&self) -> String {
        crate::a11y::snapshot(&self.accessibility_tree())
    }

    #[must_use]
    pub(crate) fn accessibility_tree_scoped(
        &self,
        scope: crate::a11y::mapping::TreeScope,
    ) -> AccessibilityTree {
        let _env = self.enter();
        crate::a11y::mapping::tree_scoped(
            &self.cx,
            self.router.focus(),
            &|id| {
                self.layout.result(id).map(|result| {
                    crate::events::hit_test::visual_bounds(&self.cx, id, result.rect())
                })
            },
            scope,
        )
    }
}
