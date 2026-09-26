use crate::kernel::Cx;

use super::diff;

const PATCH: &str = "\
@@ -1,2 +1,2 @@
 def restock(self, amount):
-    self.qty += amount
+    raise ValueError(\"amount must be positive\") from None
";

#[test]
fn a_split_diff_gives_the_longer_side_the_width() {
    let grid =
        crate::terminal::present(|cx: &mut Cx| cx.add(diff(PATCH)), 110, 12, &[]).expect("layout");
    assert!(
        grid.contains("raise ValueError(\"amount must be positive\") from None"),
        "right column cut:\n{grid}"
    );
    assert!(
        grid.contains("self.qty += amount"),
        "left column cut:\n{grid}"
    );
}

#[test]
fn an_added_line_keeps_the_split_divider_on_the_empty_side() {
    let patch = "@@ -30,2 +30,3 @@\n tools/out/\n+tools/.env\n runs/\n";
    let grid =
        crate::terminal::present(|cx: &mut Cx| cx.add(diff(patch)), 80, 10, &[]).expect("layout");
    let added = grid
        .lines()
        .find(|line| line.contains("tools/.env"))
        .expect("added row");
    assert_eq!(added.matches('│').count(), 3, "divider missing:\n{grid}");
}
