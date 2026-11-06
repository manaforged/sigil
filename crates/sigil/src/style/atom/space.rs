use std::hash::{Hash, Hasher};

/// Whether a node's back shows when it turns away, like CSS `backface-visibility`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Backface {
    /// The back shows, mirrored.
    #[default]
    Visible,
    /// The node draws nothing while its back faces the viewer.
    Hidden,
}

/// A 3D transform style atom.
#[derive(Debug, Clone, Copy)]
pub enum SpaceAtom {
    /// A turn around the x axis in radians.
    RotateX(f32),
    /// A turn around the y axis in radians.
    RotateY(f32),
    /// A turn around the z axis in radians.
    RotateZ(f32),
    /// A move along the z axis in px.
    TranslateZ(f32),
    /// The perspective distance this node gives its children; `None` removes it.
    Perspective(Option<f32>),
    /// The perspective origin in ten-thousandths of the width and height.
    PerspectiveOrigin([u16; 2]),
    /// The backface visibility.
    Backface(Backface),
}

/// The resolved 3D transform style of one node.
#[derive(Debug, Clone, Copy)]
pub struct SpaceStyle {
    /// The x, y, and z turns in radians.
    pub turn: [f32; 3],
    /// The z move in px.
    pub depth: f32,
    /// The perspective distance this node gives its children.
    pub perspective: Option<f32>,
    /// The perspective origin in ten-thousandths.
    pub perspective_origin: [u16; 2],
    /// The backface visibility.
    pub backface: Backface,
}

impl SpaceStyle {
    /// No 3D transform, perspective, or hidden backface.
    pub const FLAT: Self = Self {
        turn: [0.0; 3],
        depth: 0.0,
        perspective: None,
        perspective_origin: [5_000, 5_000],
        backface: Backface::Visible,
    };

    pub(crate) fn apply(&mut self, atom: SpaceAtom) {
        match atom {
            SpaceAtom::RotateX(radians) => self.turn[0] = radians,
            SpaceAtom::RotateY(radians) => self.turn[1] = radians,
            SpaceAtom::RotateZ(radians) => self.turn[2] = radians,
            SpaceAtom::TranslateZ(px) => self.depth = px,
            SpaceAtom::Perspective(distance) => self.perspective = distance,
            SpaceAtom::PerspectiveOrigin(origin) => self.perspective_origin = origin,
            SpaceAtom::Backface(backface) => self.backface = backface,
        }
    }

    /// Whether the node itself has no 3D turn or depth.
    #[must_use]
    pub fn is_flat(&self) -> bool {
        self.turn == [0.0; 3] && self.depth == 0.0
    }

    fn key(&self) -> [u32; 7] {
        let bits = |value: f32| if value == 0.0 { 0 } else { value.to_bits() };
        [
            bits(self.turn[0]),
            bits(self.turn[1]),
            bits(self.turn[2]),
            bits(self.depth),
            self.perspective.map_or(u32::MAX, bits),
            u32::from(self.perspective_origin[0]) << 16 | u32::from(self.perspective_origin[1]),
            u32::from(self.backface == Backface::Hidden),
        ]
    }
}

impl Default for SpaceStyle {
    fn default() -> Self {
        Self::FLAT
    }
}

impl PartialEq for SpaceStyle {
    fn eq(&self, other: &Self) -> bool {
        self.key() == other.key()
    }
}

impl Eq for SpaceStyle {}

impl Hash for SpaceStyle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key().hash(state);
    }
}

impl PartialEq for SpaceAtom {
    fn eq(&self, other: &Self) -> bool {
        let mut a = SpaceStyle::FLAT;
        let mut b = SpaceStyle::FLAT;
        a.apply(*self);
        b.apply(*other);
        std::mem::discriminant(self) == std::mem::discriminant(other) && a == b
    }
}

impl Eq for SpaceAtom {}

impl Hash for SpaceAtom {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        let mut style = SpaceStyle::FLAT;
        style.apply(*self);
        style.hash(state);
    }
}
