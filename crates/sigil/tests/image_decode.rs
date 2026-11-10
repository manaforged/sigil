#![allow(clippy::unwrap_used, clippy::panic)]

#[cfg(feature = "image-decode")]
mod decode {
    use std::io::Cursor;

    use sigil::kernel::{Cx, ImageSource, NodeType, image};

    fn png(w: u32, h: u32, rgba: [u8; 4]) -> Vec<u8> {
        let pixels = rgba.repeat((w * h) as usize);
        let img = image::RgbaImage::from_raw(w, h, pixels).unwrap();
        let mut bytes = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
            .unwrap();
        bytes
    }

    #[test]
    fn decodes_png_to_image_node_with_correct_dimensions() {
        let bytes = png(4, 3, [10, 20, 30, 255]);
        let builder = image(ImageSource::decode(&bytes).expect("valid PNG must decode"));

        let mut cx = Cx::new_root();
        let id = cx.add(builder);
        match &cx.get(id).unwrap().kind {
            NodeType::Image { source, .. } => {
                assert_eq!(source.width(), 4);
                assert_eq!(source.height(), 3);
            }
            other => panic!("expected an image node, got {other:?}"),
        }
    }

    #[test]
    fn rejects_garbage_bytes() {
        assert!(ImageSource::decode(&[0xDE, 0xAD, 0xBE, 0xEF]).is_none());
    }
}
