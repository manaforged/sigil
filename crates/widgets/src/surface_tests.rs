use sigil_kernel::{Cx, Surface};
use sigil_reactive::Runtime;

use super::surface;

#[test]
fn surface_attaches_key() {
    Runtime::scope("surface-key", || {
        let mut cx = Cx::new_root();
        let id = cx.add(surface(7));
        let key = cx
            .get(id)
            .and_then(|node| node.get_extension::<Surface>().map(|hole| hole.key));
        assert_eq!(key, Some(7));
    });
}
