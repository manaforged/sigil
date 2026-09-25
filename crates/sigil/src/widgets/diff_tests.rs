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
