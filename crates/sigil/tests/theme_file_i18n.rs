use std::str::FromStr;

use sigil::style::Theme;

#[test]
fn an_unknown_label_name_rejects_the_file() {
    let theme = Theme::from_str("base = \"light\"\n[tokens]\n[strings]\nnot_a_label = \"x\"\n");
    assert!(theme.is_err());
}
