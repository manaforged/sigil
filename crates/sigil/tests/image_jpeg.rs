use sigil::prelude::ImageSource;

const JPEG: &[u8] = include_bytes!("fixtures/images/red-4x4.jpg");

#[test]
fn image_source_decodes_a_jpeg() {
    assert!(
        ImageSource::decode(JPEG).is_some(),
        "JPEG bytes did not decode"
    );
}
