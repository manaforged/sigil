#![allow(clippy::cast_lossless)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::indexing_slicing)]
#![allow(clippy::redundant_pub_crate)]
#![allow(clippy::too_many_arguments)]

pub(super) use sigil_kittest::gpu::{create_capture_texture, pixel_rgba};
pub(super) use sigil_kittest::gpu::{read_pixels, shared_device as try_request_device};

pub(super) fn abs_diff(a: u8, b: u8) -> u8 {
    (a as i16 - b as i16).unsigned_abs() as u8
}
