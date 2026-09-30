use super::*;

const REGULAR: &[u8] = sigil_fonts::FIRA_SANS_REGULAR;
const BOLD: &[u8] = include_bytes!("../../tests/fixtures/text/FiraSans-Bold.ttf");

#[test]
fn static_family_selects_the_css_weight_match() {
    let regular = Font::from_bytes(REGULAR.to_vec(), 0).expect("regular fixture");
    let bold = Font::from_bytes(BOLD.to_vec(), 0).expect("bold fixture");
    let regular_id = regular.id();
    let bold_id = bold.id();
    let mut family = FontFamily::new(regular);
    family.insert(700, bold);

    assert_eq!(family.select(400).id(), regular_id);
    assert_eq!(family.select(600).id(), bold_id);
    assert_eq!(family.select(700).id(), bold_id);
    assert_eq!(family.select(800).id(), bold_id);
}
