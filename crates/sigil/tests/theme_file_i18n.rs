#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::expect_used)]

use std::str::FromStr;

use sigil::style::{DateOrder, Direction, StringKey, Theme};

#[test]
fn a_theme_file_sets_direction_locale_and_labels() {
    let theme = Theme::from_str(
        r#"
base = "light"
direction = "rtl"

[tokens]

[locale]
preset = "de"
date_order = "ymd"

[strings]
close = "Schließen"
"#,
    )
    .expect("theme parses");
    assert_eq!(theme.direction, Direction::Rtl);
    assert_eq!(theme.locale.decimal, ',');
    assert_eq!(theme.locale.date_order, DateOrder::Ymd);
    assert_eq!(theme.strings.get(StringKey::Close), "Schließen");
}

#[test]
fn an_unknown_label_name_rejects_the_file() {
    let theme = Theme::from_str("base = \"light\"\n[tokens]\n[strings]\nnot_a_label = \"x\"\n");
    assert!(theme.is_err());
}
