use std::borrow::Cow;

use sigil::geometry::PhysicalSize;
use sigil::scene::{CommandScene, SceneCommand};
use sigil::style::Color;

mod draws;
mod shapes;
pub mod stress;

pub const SCALES: [f32; 5] = [1.0, 1.25, 1.5, 2.0, 3.0];
const SEED_BYTES: usize = 16_384;
const MAX_DEPTH: u32 = 6;

pub struct Entropy<'a> {
    bytes: Cow<'a, [u8]>,
    at: usize,
}

impl<'a> Entropy<'a> {
    #[must_use]
    pub const fn from_bytes(bytes: &'a [u8]) -> Self {
        Self {
            bytes: Cow::Borrowed(bytes),
            at: 0,
        }
    }

    #[must_use]
    pub fn from_seed(seed: u64) -> Entropy<'static> {
        let mut state = seed;
        let mut bytes = Vec::with_capacity(SEED_BYTES);
        while bytes.len() < SEED_BYTES {
            state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            bytes.extend_from_slice(&(z ^ (z >> 31)).to_le_bytes());
        }
        Entropy {
            bytes: Cow::Owned(bytes),
            at: 0,
        }
    }

    pub fn byte(&mut self) -> u8 {
        let value = self.bytes.get(self.at).copied().unwrap_or(0);
        self.at += 1;
        value
    }

    #[must_use]
    pub fn exhausted(&self) -> bool {
        self.at >= self.bytes.len()
    }

    pub fn below(&mut self, bound: u32) -> u32 {
        let raw = u32::from(u16::from_le_bytes([self.byte(), self.byte()]));
        raw % bound.max(1)
    }

    pub fn chance(&mut self, percent: u8) -> bool {
        self.byte() % 100 < percent
    }

    pub fn unit(&mut self) -> f32 {
        f32::from(u16::from_le_bytes([self.byte(), self.byte()])) / f32::from(u16::MAX)
    }

    pub fn range(&mut self, low: f32, high: f32) -> f32 {
        (high - low).mul_add(self.unit(), low)
    }

    pub fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        let len = u32::try_from(items.len()).unwrap_or(u32::MAX);
        let index = usize::try_from(self.below(len)).unwrap_or(0);
        items
            .get(index)
            .or_else(|| items.last())
            .copied()
            .expect("pick needs at least one item")
    }
}

#[derive(Clone, Debug)]
pub struct GeneratedScene {
    pub commands: Vec<SceneCommand>,
    pub viewport: (f32, f32),
    pub scale: f32,
    pub clear: Color,
}

impl GeneratedScene {
    #[must_use]
    pub fn from_seed(seed: u64) -> Self {
        generate(&mut Entropy::from_seed(seed))
    }

    #[must_use]
    pub fn scene(&self) -> CommandScene {
        CommandScene {
            commands: self.commands.clone(),
        }
    }

    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn physical(&self) -> PhysicalSize {
        PhysicalSize::new(
            (self.viewport.0 * self.scale).round() as u32,
            (self.viewport.1 * self.scale).round() as u32,
        )
    }

    #[must_use]
    pub fn keep(&self, indices: &[usize]) -> Self {
        Self {
            commands: indices
                .iter()
                .filter_map(|&i| self.commands.get(i).cloned())
                .collect(),
            ..self.clone()
        }
    }

    #[must_use]
    pub fn from_fixture(text: &str) -> Option<Self> {
        let mut base = None;
        let mut keep = None;
        for line in text.lines() {
            match line.split_once(' ') {
                Some(("seed", value)) => base = Some(Self::from_seed(value.trim().parse().ok()?)),
                Some(("bytes", hex)) => {
                    base = Some(generate(&mut Entropy::from_bytes(&unhex(hex)?)));
                }
                Some(("keep", list)) => {
                    keep = Some(
                        list.split_whitespace()
                            .map(str::parse)
                            .collect::<Result<Vec<usize>, _>>()
                            .ok()?,
                    );
                }
                _ => {}
            }
        }
        let base = base?;
        Some(keep.map_or_else(|| base.clone(), |keep| base.keep(&keep)))
    }
}

#[must_use]
pub fn fixture_text(seed: u64, keep: &[usize]) -> String {
    let list: Vec<String> = keep.iter().map(ToString::to_string).collect();
    format!("seed {seed}\nkeep {}\n", list.join(" "))
}

fn unhex(hex: &str) -> Option<Vec<u8>> {
    let hex = hex.trim().as_bytes();
    hex.chunks(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok())
        .collect()
}

#[must_use]
pub fn generate(entropy: &mut Entropy<'_>) -> GeneratedScene {
    let width = f32::from(u16::try_from(entropy.below(64) + 1).unwrap_or(1)) * 4.0;
    let height = f32::from(u16::try_from(entropy.below(64) + 1).unwrap_or(1)) * 4.0;
    let scale = entropy.pick(&SCALES);
    let clear = Color::rgba(entropy.unit(), entropy.unit(), entropy.unit(), 1.0);
    let mut out = Vec::new();
    let budget = entropy.below(48) + 1;
    let mut builder = Gen {
        entropy,
        viewport: (width, height),
        out: &mut out,
        budget,
    };
    builder.body(0);
    GeneratedScene {
        commands: out,
        viewport: (width, height),
        scale,
        clear,
    }
}

pub(crate) struct Gen<'e, 'b, 'o> {
    pub(crate) entropy: &'e mut Entropy<'b>,
    pub(crate) viewport: (f32, f32),
    pub(crate) out: &'o mut Vec<SceneCommand>,
    budget: u32,
}

impl Gen<'_, '_, '_> {
    fn body(&mut self, depth: u32) {
        let items = self.entropy.below(6) + 1;
        for _ in 0..items {
            if self.budget == 0 || self.entropy.exhausted() {
                return;
            }
            self.budget -= 1;
            self.item(depth);
        }
    }

    fn item(&mut self, depth: u32) {
        let nest = depth < MAX_DEPTH;
        match self.entropy.below(20) {
            0 | 1 if nest => {
                let clip = self.rect();
                self.nested(SceneCommand::PushClip(clip), SceneCommand::PopClip, depth);
            }
            2 | 3 if nest => {
                let transform = self.transform();
                self.nested(
                    SceneCommand::PushTransform(transform),
                    SceneCommand::PopTransform,
                    depth,
                );
            }
            4 if nest => {
                let opacity = self.opacity_f32();
                self.nested(
                    SceneCommand::SaveLayer { opacity },
                    SceneCommand::RestoreLayer,
                    depth,
                );
            }
            5 if nest => {
                let scope = self.layer_scope();
                self.nested(
                    SceneCommand::PushLayer(scope),
                    SceneCommand::PopLayer,
                    depth,
                );
            }
            6 => self.stray_pop(),
            _ => self.draw(),
        }
    }

    fn nested(&mut self, open: SceneCommand, close: SceneCommand, depth: u32) {
        self.out.push(open);
        if !self.entropy.chance(8) {
            self.body(depth + 1);
        }
        if !self.entropy.chance(4) {
            self.out.push(close);
        }
    }

    fn stray_pop(&mut self) {
        let pop = match self.entropy.below(4) {
            0 => SceneCommand::PopClip,
            1 => SceneCommand::PopTransform,
            2 => SceneCommand::RestoreLayer,
            _ => SceneCommand::PopLayer,
        };
        self.out.push(pop);
    }
}
