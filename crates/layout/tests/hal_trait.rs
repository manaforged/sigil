#![allow(unused_crate_dependencies)]
#![allow(missing_docs)]
#![allow(clippy::float_cmp)]
#![allow(clippy::unwrap_used)]

use ahash as _;
use allocation_counter as _;
use criterion as _;
use rayon as _;
use sigil_reactive as _;
use sigil_style as _;
use smallvec as _;
mod stub;
use stub::stub_engine;

use sigil_hal::{Layout, Size};
use sigil_kernel::prelude::*;
use sigil_layout::prelude::*;

#[test]
fn raw_hal_consumer_can_drive_layout_engine() {
    let mut cx = Cx::new_root();
    let root = cx.add(container());
    let child = cx.add(container());

    let mut engine = stub_engine();
    layout_two_boxes(&mut engine, root, child).expect("hal layout succeeds");

    let child_result = engine.result(child).expect("child result exists");
    assert_eq!(child_result.rect().width, 10.0);
    assert_eq!(child_result.rect().height, 8.0);
}

fn layout_two_boxes<L>(
    engine: &mut L,
    root: NodeId,
    child: NodeId,
) -> Result<(), <L as Layout>::Error>
where
    L: Layout<NodeId = NodeId, Intent = LayoutStyle, Result = LayoutResult>,
{
    engine.upsert(root, &LayoutStyle::flex_row().fixed_size(100.0, 20.0))?;
    engine.upsert(child, &LayoutStyle::default().fixed_size(10.0, 8.0))?;
    engine.set_children(root, &[child])?;
    engine.compute(root, Size::fixed(100.0, 20.0))
}
