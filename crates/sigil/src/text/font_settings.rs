#![allow(clippy::redundant_pub_crate)]

pub(crate) const AXES: usize = 6;
pub(crate) const FEATURES: usize = 6;

const FIXED_ONE: f32 = 65_536.0;

pub(crate) type Tag = [u8; 4];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct TagSet<V, const N: usize> {
    len: u8,
    items: [(Tag, V); N],
}

impl<V: Copy, const N: usize> TagSet<V, N> {
    const fn empty(zero: V) -> Self {
        Self {
            len: 0,
            items: [([0; 4], zero); N],
        }
    }

    #[cfg(feature = "scene")]
    pub(crate) fn iter(&self) -> impl Iterator<Item = (Tag, V)> + '_ {
        self.items.iter().take(usize::from(self.len)).copied()
    }

    pub(crate) fn set(&mut self, tag: Tag, value: V) {
        let len = usize::from(self.len);
        let live = self.items.get_mut(..len).unwrap_or_default();
        let at = match live.binary_search_by(|item| item.0.cmp(&tag)) {
            Ok(found) => {
                if let Some(item) = live.get_mut(found) {
                    item.1 = value;
                }
                return;
            }
            Err(at) => at,
        };
        if len >= N {
            return;
        }
        if let Some(open) = self.items.get_mut(at..=len) {
            open.rotate_right(1);
            if let Some(first) = open.first_mut() {
                *first = (tag, value);
            }
            self.len = self.len.saturating_add(1);
        }
    }

    #[cfg(feature = "scene")]
    fn over(self, base: Self) -> Self {
        let mut out = base;
        for (tag, value) in self.iter() {
            out.set(tag, value);
        }
        out
    }
}

pub(crate) type Axes = TagSet<i32, AXES>;
pub(crate) type Features = TagSet<u16, FEATURES>;

impl Axes {
    pub(crate) const EMPTY: Self = Self::empty(0);

    #[cfg(feature = "scene")]
    #[allow(
        clippy::cast_precision_loss,
        reason = "a 16.16 fixed-point axis value converts back to f32 the way OpenType defines it"
    )]
    pub(crate) fn values(&self) -> impl Iterator<Item = (Tag, f32)> + '_ {
        self.iter()
            .map(|(tag, fixed)| (tag, fixed as f32 / FIXED_ONE))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct FontSettings {
    pub(crate) axes: Axes,
    pub(crate) features: Features,
}

impl FontSettings {
    pub(crate) const fn new() -> Self {
        Self {
            axes: Axes::EMPTY,
            features: Features::empty(0),
        }
    }

    pub(crate) fn axis(&mut self, tag: &str, value: f32) {
        if let Some((tag, value)) = parse_axis(tag, value) {
            self.axes.set(tag, value);
        }
    }

    pub(crate) fn feature(&mut self, tag: &str, value: u16) {
        if let Some(tag) = parse_tag(tag) {
            self.features.set(tag, value);
        }
    }

    #[cfg(feature = "scene")]
    pub(crate) fn over(self, base: Self) -> Self {
        Self {
            axes: self.axes.over(base.axes),
            features: self.features.over(base.features),
        }
    }
}

impl Default for FontSettings {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn parse_axis(tag: &str, value: f32) -> Option<(Tag, i32)> {
    Some((parse_tag(tag)?, fixed(value)?))
}

pub(crate) fn parse_tag(tag: &str) -> Option<Tag> {
    let tag: Tag = tag.as_bytes().try_into().ok()?;
    tag.iter()
        .all(|byte| (0x20..=0x7e).contains(byte))
        .then_some(tag)
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "a float-to-int cast saturates, and no font axis reaches the i32 range"
)]
fn fixed(value: f32) -> Option<i32> {
    value
        .is_finite()
        .then(|| (value * FIXED_ONE).round() as i32)
}
