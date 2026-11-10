pub(crate) const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

const PRIME: u64 = 0x0000_0100_0000_01b3;

pub(crate) const fn mix(hash: u64, value: u64) -> u64 {
    (hash ^ value).wrapping_mul(PRIME)
}
