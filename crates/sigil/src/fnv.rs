pub(crate) const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

const PRIME: u64 = 0x0000_0100_0000_01b3;

pub(crate) const fn mix(hash: u64, value: u64) -> u64 {
    (hash ^ value).wrapping_mul(PRIME)
}

pub(crate) fn mix_f32(hash: u64, value: f32) -> u64 {
    mix(hash, u64::from(value.to_bits()))
}

pub(crate) fn bytes(hash: u64, data: &[u8]) -> u64 {
    data.iter()
        .fold(hash, |hash, byte| mix(hash, u64::from(*byte)))
}
