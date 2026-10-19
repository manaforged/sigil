use super::diff;

const PATCH: &str = "\
@@ -1,2 +1,2 @@
 def restock(self, amount):
-    self.qty += amount
+    raise ValueError(\"amount must be positive\") from None
";

#[test]
fn a_split_diff_gives_the_longer_side_the_width() {
    let grid = crate::terminal::present(|| diff(PATCH), 110, 12, &[]).expect("layout");
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
    let grid = crate::terminal::present(move || diff(patch), 80, 10, &[]).expect("layout");
    let added = grid
        .lines()
        .find(|line| line.contains("tools/.env"))
        .expect("added row");
    assert_eq!(added.matches('│').count(), 3, "divider missing:\n{grid}");
}

#[test]
fn a_cut_changed_line_ends_in_one_ellipsis() {
    let patch = "@@ -1,2 +1,2 @@\n def add(a, b):\n-    return a - b\n+    return a + b\n";
    let grid = crate::terminal::present(move || diff(patch), 44, 8, &[]).expect("layout");
    assert!(grid.contains("return"), "changed line missing:\n{grid}");
    assert!(!grid.contains("……"), "doubled ellipsis:\n{grid}");
}

#[test]
fn a_wrapped_diff_keeps_the_end_of_a_long_line() {
    let patch = "@@ -1,1 +1,1 @@\n-    total = 0\n+    total = sum(row.cents for row in rows if row.category == wanted and row.month == month_tail)\n";
    let wrapped = crate::terminal::present(
        move || diff(patch).view(super::DiffView::Unified).wrap(true),
        48,
        14,
        &[],
    )
    .expect("layout");
    assert!(wrapped.contains("month_tail"), "line cut:\n{wrapped}");
    assert!(!wrapped.contains('…'), "line ellipsized:\n{wrapped}");
}
