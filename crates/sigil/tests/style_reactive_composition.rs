#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::panic, clippy::unwrap_used)]

use sigil::kernel::*;
use sigil::reactive::Runtime;
use sigil::style::{NodeStyleExt, Px, StyleAtom, StyleBinding};

static FIRST: &[StyleAtom] = &[StyleAtom::Opacity(5_000)];
static SECOND: &[StyleAtom] = &[StyleAtom::Opacity(9_000), StyleAtom::BorderRadius(Px(4))];

#[test]
fn multiple_reactive_sources_compose_in_install_order() {
    Runtime::scope("style_reactive_composition", || {
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .style_reactive(|| FIRST)
                .style_reactive(|| SECOND),
        );
        let binding = cx
            .get(root)
            .and_then(|node| node.get_extension::<StyleBinding>())
            .unwrap();
        assert!(binding.has_reactive());
        let channels: Vec<&'static [StyleAtom]> = binding.reactive_channels().collect();
        assert_eq!(channels.len(), 2, "second install must not clobber");
        assert_eq!(channels.first(), Some(&FIRST));
        assert_eq!(channels.get(1), Some(&SECOND));
    });
}

#[test]
fn composition_is_append_only_across_many_installs() {
    Runtime::scope("style_reactive_composition_many", || {
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .style_reactive(|| FIRST)
                .style_reactive(|| FIRST)
                .style_reactive(|| SECOND),
        );
        let binding = cx
            .get(root)
            .and_then(|node| node.get_extension::<StyleBinding>())
            .unwrap();
        let channels: Vec<&'static [StyleAtom]> = binding.reactive_channels().collect();
        assert_eq!(channels.len(), 3);
        assert_eq!(channels.get(2), Some(&SECOND));
    });
}
