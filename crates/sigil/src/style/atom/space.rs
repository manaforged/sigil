use std::hash::{Hash, Hasher};

use crate::style::{AnimatorOverlay, NumericAtomKind};

kittest_pub! {
    #[doc(hidden)]
    /// Whether a node's back shows when it turns away, like CSS `backface-visibility`.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub enum Backface {
        /// The back shows, mirrored.
        #[default]
        Visible,
        /// The node draws nothing while its back faces the viewer.
        Hidden,
    }
}

kittest_pub! {
    #[doc(hidden)]
    /// A 2D and 3D transform style atom.
    #[derive(Debug, Clone, Copy)]
    pub enum SpaceAtom {
        /// A horizontal move in pixels.
        TranslateX(f32),
        /// A vertical move in pixels.
        TranslateY(f32),
        /// A horizontal scale factor.
        ScaleX(f32),
        /// A vertical scale factor.
        ScaleY(f32),
        /// A 2D rotation in radians.
        Rotate(f32),
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
        /// The point that scale and rotate turn around, in ten-thousandths of the width and height.
        TransformOrigin([u16; 2]),
        /// Whether the node draws while its back faces the viewer.
        Backface(Backface),
        /// Whether the children stay in this node's 3D space.
        Preserve3d(bool),
    }
}

kittest_pub! {
    /// The resolved 3D transform style of one node.
    #[derive(Debug, Clone, Copy)]
    pub struct SpaceStyle {
        /// The x, y, and z turns in radians.
        pub turn: [f32; 3],
        /// The move along the z axis in pixels; the default is 0.
        pub depth: f32,
        /// The perspective distance this node gives its children.
        pub perspective: Option<f32>,
        /// The perspective origin in ten-thousandths.
        pub perspective_origin: [u16; 2],
        /// Whether the node draws while its back faces the viewer; the default is visible.
        pub backface: Backface,
        /// Whether the children stay in this node's 3D space; the default is flat.
        pub preserve: bool,
    }
}

impl SpaceStyle {
    /// No 3D transform, perspective, or hidden backface.
    pub(crate) const FLAT: Self = Self {
        turn: [0.0; 3],
        depth: 0.0,
        perspective: None,
        perspective_origin: [5_000, 5_000],
        backface: Backface::Visible,
        preserve: false,
    };

    pub(crate) const fn apply(&mut self, atom: SpaceAtom) {
        match atom {
            SpaceAtom::TranslateX(_)
            | SpaceAtom::TranslateY(_)
            | SpaceAtom::ScaleX(_)
            | SpaceAtom::ScaleY(_)
            | SpaceAtom::Rotate(_)
            | SpaceAtom::TransformOrigin(_) => {}
            SpaceAtom::RotateX(radians) => self.turn[0] = radians,
            SpaceAtom::RotateY(radians) => self.turn[1] = radians,
            SpaceAtom::RotateZ(radians) => self.turn[2] = radians,
            SpaceAtom::TranslateZ(px) => self.depth = px,
            SpaceAtom::Perspective(distance) => self.perspective = distance,
            SpaceAtom::PerspectiveOrigin(origin) => self.perspective_origin = origin,
            SpaceAtom::Backface(backface) => self.backface = backface,
            SpaceAtom::Preserve3d(on) => self.preserve = on,
        }
    }

    /// Whether the node itself has no 3D turn or depth.
    #[must_use]
    pub(crate) fn is_flat(&self) -> bool {
        self.turn.iter().all(|turn| *turn == 0.0) && self.depth == 0.0
    }

    pub(crate) const fn value(&self, kind: NumericAtomKind) -> Option<f32> {
        match kind {
            NumericAtomKind::RotationX => Some(self.turn[0]),
            NumericAtomKind::RotationY => Some(self.turn[1]),
            NumericAtomKind::RotationZ => Some(self.turn[2]),
            NumericAtomKind::TranslateZ => Some(self.depth),
            NumericAtomKind::Perspective => self.perspective,
            _ => None,
        }
    }

    pub(crate) fn overlay(&mut self, overlay: &dyn AnimatorOverlay) {
        let [x, y, z] = &mut self.turn;
        for (kind, slot) in [
            (NumericAtomKind::RotationX, x),
            (NumericAtomKind::RotationY, y),
            (NumericAtomKind::RotationZ, z),
            (NumericAtomKind::TranslateZ, &mut self.depth),
        ] {
            if let Some(value) = overlay.value_for(kind) {
                *slot = value;
            }
        }
        if self.perspective.is_some()
            && let Some(px) = overlay.value_for(NumericAtomKind::Perspective)
        {
            self.perspective = Some(px.max(1.0));
        }
    }

    fn key(&self) -> [u32; 8] {
        [
            f32_bits(self.turn[0]),
            f32_bits(self.turn[1]),
            f32_bits(self.turn[2]),
            f32_bits(self.depth),
            self.perspective.map_or(u32::MAX, f32_bits),
            u32::from(self.perspective_origin[0]) << 16 | u32::from(self.perspective_origin[1]),
            u32::from(self.backface == Backface::Hidden),
            u32::from(self.preserve),
        ]
    }
}

fn f32_bits(value: f32) -> u32 {
    if value == 0.0 { 0 } else { value.to_bits() }
}

impl SpaceAtom {
    fn key(self) -> (u8, u64) {
        match self {
            Self::TranslateX(px) => (0, u64::from(f32_bits(px))),
            Self::TranslateY(px) => (1, u64::from(f32_bits(px))),
            Self::ScaleX(factor) => (2, u64::from(f32_bits(factor))),
            Self::ScaleY(factor) => (3, u64::from(f32_bits(factor))),
            Self::Rotate(radians) => (4, u64::from(f32_bits(radians))),
            Self::RotateX(radians) => (5, u64::from(f32_bits(radians))),
            Self::RotateY(radians) => (6, u64::from(f32_bits(radians))),
            Self::RotateZ(radians) => (7, u64::from(f32_bits(radians))),
            Self::TranslateZ(px) => (8, u64::from(f32_bits(px))),
            Self::Perspective(Some(distance)) => (9, u64::from(f32_bits(distance))),
            Self::Perspective(None) => (10, 0),
            Self::PerspectiveOrigin([x, y]) => (11, u64::from(x) << 16 | u64::from(y)),
            Self::Backface(Backface::Visible) => (12, 0),
            Self::Backface(Backface::Hidden) => (12, 1),
            Self::Preserve3d(on) => (13, u64::from(on)),
            Self::TransformOrigin([x, y]) => (14, u64::from(x) << 16 | u64::from(y)),
        }
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
        self.key() == other.key()
    }
}

impl Eq for SpaceAtom {}

impl Hash for SpaceAtom {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key().hash(state);
    }
}
