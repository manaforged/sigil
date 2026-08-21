use super::*;

use sigil_reactive::Runtime;

#[test]
fn sheet_docks_a_dialog_on_the_right() {
    Runtime::scope("sheet-right", || {
        let mut cx = Cx::new_root();
        let show = signal(true);
        let root = cx.add(sheet(show).title("Filters"));
        let frame = cx
            .get(root)
            .expect("root")
            .children()
            .iter()
            .copied()
            .find(|id| matches!(cx.get(*id).expect("child").a11y.role, Role::Dialog))
            .expect("dialog");
        let style = cx
            .get(frame)
            .expect("frame")
            .get_extension::<LayoutStyle>()
            .expect("layout")
            .clone();
        assert_eq!(
            style.data().size.width,
            sigil_layout::prelude::Dimension::Px(360.0)
        );
        assert_eq!(
            style.data().size.height,
            sigil_layout::prelude::Dimension::Fill
        );
    });
}
