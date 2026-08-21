use skrifa::{FontRef, MetadataProvider};
use smallvec::SmallVec;

pub(super) type VariationSettings = SmallVec<[([u8; 4], f32); 8]>;

#[derive(Debug, Clone)]
pub(super) struct VariationModel {
    axes: Box<[Axis]>,
    instances: Box<[Instance]>,
    weight_axis: Option<usize>,
}

#[derive(Debug, Clone, Copy)]
struct Axis {
    tag: [u8; 4],
    default: f32,
    hidden: bool,
}

#[derive(Debug, Clone)]
struct Instance {
    coords: Box<[f32]>,
}

impl VariationModel {
    pub(super) fn new(font: &FontRef<'_>) -> Self {
        let axes = font
            .axes()
            .iter()
            .map(|axis| Axis {
                tag: axis.tag().to_be_bytes(),
                default: axis.default_value(),
                hidden: axis.is_hidden(),
            })
            .collect::<Box<[_]>>();
        let instances = font
            .named_instances()
            .iter()
            .map(|instance| Instance {
                coords: instance.user_coords().collect(),
            })
            .collect();
        let weight_axis = axes.iter().position(|axis| axis.tag == *b"wght");
        Self {
            axes,
            instances,
            weight_axis,
        }
    }

    pub(super) fn settings(&self, weight: u16, size_px: f32) -> VariationSettings {
        let target_weight = f32::from(weight.clamp(1, 1000));
        let mut settings = self.hidden_axis_settings(target_weight);
        self.set_if_present(&mut settings, *b"wght", target_weight);
        self.set_if_present(&mut settings, *b"opsz", size_px.max(1.0));
        settings
    }

    fn hidden_axis_settings(&self, target_weight: f32) -> VariationSettings {
        let Some(weight_axis) = self.weight_axis else {
            return VariationSettings::new();
        };
        let (lower, upper) = self.weight_bracket(weight_axis, target_weight);
        let mut settings = VariationSettings::new();
        for (index, axis) in self.axes.iter().enumerate().filter(|(_, axis)| axis.hidden) {
            let value = interpolate_axis(lower, upper, index, weight_axis, target_weight);
            if let Some(value) = value {
                settings.push((axis.tag, value));
            }
        }
        settings
    }

    fn weight_bracket(
        &self,
        weight_axis: usize,
        target_weight: f32,
    ) -> (Option<&Instance>, Option<&Instance>) {
        let mut lower: Option<&Instance> = None;
        let mut upper: Option<&Instance> = None;
        for instance in self
            .instances
            .iter()
            .filter(|item| self.is_default_width(item))
        {
            let Some(&weight) = instance.coords.get(weight_axis) else {
                continue;
            };
            if weight <= target_weight
                && lower
                    .and_then(|current| current.coords.get(weight_axis))
                    .is_none_or(|current| *current < weight)
            {
                lower = Some(instance);
            }
            if weight >= target_weight
                && upper
                    .and_then(|current| current.coords.get(weight_axis))
                    .is_none_or(|current| *current > weight)
            {
                upper = Some(instance);
            }
        }
        (lower.or(upper), upper.or(lower))
    }

    fn is_default_width(&self, instance: &Instance) -> bool {
        self.axes.iter().enumerate().all(|(index, axis)| {
            axis.hidden
                || matches!(&axis.tag, b"wght" | b"opsz")
                || instance
                    .coords
                    .get(index)
                    .is_none_or(|value| (value - axis.default).abs() < 0.001)
        })
    }

    fn set_if_present(&self, settings: &mut VariationSettings, tag: [u8; 4], value: f32) {
        if self.axes.iter().any(|axis| axis.tag == tag) {
            if let Some(setting) = settings.iter_mut().find(|setting| setting.0 == tag) {
                setting.1 = value;
            } else {
                settings.push((tag, value));
            }
        }
    }
}

fn interpolate_axis(
    lower: Option<&Instance>,
    upper: Option<&Instance>,
    axis: usize,
    weight_axis: usize,
    target_weight: f32,
) -> Option<f32> {
    let lower = lower?;
    let upper = upper?;
    let lower_weight = *lower.coords.get(weight_axis)?;
    let upper_weight = *upper.coords.get(weight_axis)?;
    let lower_value = *lower.coords.get(axis)?;
    let upper_value = *upper.coords.get(axis)?;
    if (upper_weight - lower_weight).abs() < f32::EPSILON {
        return Some(lower_value);
    }
    let fraction = (target_weight - lower_weight) / (upper_weight - lower_weight);
    Some((upper_value - lower_value).mul_add(fraction.clamp(0.0, 1.0), lower_value))
}

#[cfg(test)]
#[path = "variations_tests.rs"]
mod tests;
