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

struct Fnv(u64);

impl std::hash::Hasher for Fnv {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, data: &[u8]) {
        self.0 = bytes(self.0, data);
    }
}

pub(crate) fn hash_of<T: std::hash::Hash + ?Sized>(value: &T) -> u64 {
    use std::hash::Hasher;
    let mut hasher = Fnv(OFFSET);
    value.hash(&mut hasher);
    hasher.finish()
}
