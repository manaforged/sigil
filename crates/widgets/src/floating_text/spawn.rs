use sigil_style::Rgba8;

#[derive(Clone, Debug, PartialEq)]
pub struct FctSpawn {
    pub at: [f32; 2],
    pub text: String,
    pub color: Rgba8,
}

impl FctSpawn {
    #[must_use]
    pub fn new(at: [f32; 2], text: impl Into<String>, color: Rgba8) -> Self {
        Self {
            at,
            text: text.into(),
            color,
        }
    }
}

impl Default for FctSpawn {
    fn default() -> Self {
        Self {
            at: [0.0, 0.0],
            text: String::new(),
            color: Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Particle {
    pub origin: [f32; 2],
    pub drift: [f32; 2],
    pub position: [f32; 2],
    pub spawn_time: f32,
    pub remaining: f32,
    pub alpha: f32,
    pub color: Rgba8,
    pub text: String,
    pub alive: bool,
}

impl Particle {
    pub(super) const fn empty() -> Self {
        Self {
            origin: [0.0, 0.0],
            drift: [0.0, 0.0],
            position: [0.0, 0.0],
            spawn_time: 0.0,
            remaining: 0.0,
            alpha: 0.0,
            color: Rgba8 {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
            text: String::new(),
            alive: false,
        }
    }
}
