use super::*;
use crate::testing::runtime;

#[test]
fn the_cursor_wraps_or_clamps_at_the_list_edge() {
    runtime("list-state-edge", |_| {
        let list: ListState = ListState::new();
        list.shift(-1, 3, CursorEdge::Wrap);
        assert_eq!(list.at(3), 2);
        list.shift(1, 3, CursorEdge::Clamp);
        assert_eq!(list.at(3), 2);
        list.shift(1, 3, CursorEdge::Wrap);
        assert_eq!(list.at(3), 0);
        list.move_to(9);
        assert_eq!(list.at(3), 2);
    });
}
