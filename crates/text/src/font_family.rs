//! CSS-weight-aware face selection within one font family.

use crate::shaper::Font;

/// One normal-style font family, possibly backed by static weight faces.
#[derive(Clone)]
pub(crate) struct FontFamily {
    faces: Vec<WeightedFace>,
}

#[derive(Clone)]
struct WeightedFace {
    weight: u16,
    font: Font,
}

impl FontFamily {
    pub(crate) fn new(font: Font) -> Self {
        Self {
            faces: vec![WeightedFace { weight: 400, font }],
        }
    }

    pub(crate) fn from_weighted(faces: Vec<(u16, Font)>) -> Option<Self> {
        let mut family = Self { faces: Vec::new() };
        for (weight, font) in faces {
            family.insert(weight, font);
        }
        (!family.faces.is_empty()).then_some(family)
    }

    pub(crate) fn select(&self, requested: u16) -> &Font {
        if let Some(face) = self.faces.iter().find(|face| face.font.has_weight_axis()) {
            return &face.font;
        }
        let index = match_weight(&self.faces, requested.clamp(1, 1000));
        self.faces
            .get(index)
            .map(|face| &face.font)
            .expect("FontFamily always contains at least one face")
    }

    pub(crate) fn replace(&mut self, font: Font) {
        self.faces.clear();
        self.faces.push(WeightedFace { weight: 400, font });
    }

    pub(crate) fn insert(&mut self, weight: u16, font: Font) {
        let weight = weight.clamp(1, 1000);
        if self.faces.iter().any(|face| face.font.id() == font.id()) {
            return;
        }
        if let Some(face) = self.faces.iter_mut().find(|face| face.weight == weight) {
            face.font = font;
        } else {
            self.faces.push(WeightedFace { weight, font });
            self.faces.sort_unstable_by_key(|face| face.weight);
        }
    }
}

fn match_weight(faces: &[WeightedFace], requested: u16) -> usize {
    if let Some(index) = faces.iter().position(|face| face.weight == requested) {
        return index;
    }
    if (400..450).contains(&requested)
        && let Some(index) = faces.iter().position(|face| face.weight == 500)
    {
        return index;
    }
    if (450..=500).contains(&requested)
        && let Some(index) = faces.iter().position(|face| face.weight == 400)
    {
        return index;
    }
    if requested <= 500 {
        thinner_or_heavier(faces, requested)
    } else {
        heavier_or_thinner(faces, requested)
    }
}

fn thinner_or_heavier(faces: &[WeightedFace], requested: u16) -> usize {
    faces
        .iter()
        .enumerate()
        .filter(|(_, face)| face.weight <= requested)
        .max_by_key(|(_, face)| face.weight)
        .or_else(|| faces.iter().enumerate().min_by_key(|(_, face)| face.weight))
        .map_or(0, |(index, _)| index)
}

fn heavier_or_thinner(faces: &[WeightedFace], requested: u16) -> usize {
    faces
        .iter()
        .enumerate()
        .filter(|(_, face)| face.weight >= requested)
        .min_by_key(|(_, face)| face.weight)
        .or_else(|| faces.iter().enumerate().max_by_key(|(_, face)| face.weight))
        .map_or(0, |(index, _)| index)
}

#[cfg(test)]
mod tests {
    use super::*;

    const REGULAR: &[u8] = include_bytes!("../assets/fonts/FiraSans-Regular.ttf");
    const BOLD: &[u8] = include_bytes!("../tests/fixtures/FiraSans-Bold.ttf");

    #[test]
    fn static_family_selects_the_css_weight_match() {
        let regular = Font::from_bytes(REGULAR.to_vec(), 0).expect("regular fixture");
        let bold = Font::from_bytes(BOLD.to_vec(), 0).expect("bold fixture");
        let regular_id = regular.id();
        let bold_id = bold.id();
        let mut family = FontFamily::new(regular);
        family.insert(700, bold);

        assert_eq!(family.select(400).id(), regular_id);
        assert_eq!(family.select(600).id(), bold_id);
        assert_eq!(family.select(700).id(), bold_id);
        assert_eq!(family.select(800).id(), bold_id);
    }
}
