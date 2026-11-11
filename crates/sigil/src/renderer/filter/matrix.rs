use crate::scene::command::Filter;

pub(crate) type ColorMatrix = [[f32; 5]; 4];

const IDENTITY: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
const LUMA: [f32; 3] = [0.2126, 0.7152, 0.0722];
const SATURATE_LUMA: [f32; 3] = [0.213, 0.715, 0.072];
const SEPIA: [[f32; 3]; 3] = [
    [0.393, 0.769, 0.189],
    [0.349, 0.686, 0.168],
    [0.272, 0.534, 0.131],
];
const HUE_COS: [[f32; 3]; 3] = [
    [0.787, -0.715, -0.072],
    [-0.213, 0.285, -0.072],
    [-0.213, -0.715, 0.928],
];
const HUE_SIN: [[f32; 3]; 3] = [
    [-0.213, -0.715, 0.928],
    [0.143, 0.140, -0.283],
    [-0.787, 0.715, 0.072],
];

pub(crate) fn color_matrix(filter: Filter) -> Option<ColorMatrix> {
    let unit = |amount: f32| amount.clamp(0.0, 1.0);
    Some(match filter {
        Filter::Blur(_) | Filter::DropShadow(_) => return None,
        Filter::Brightness(amount) => transfer(amount.max(0.0), 0.0),
        Filter::Contrast(amount) => {
            let slope = amount.max(0.0);
            transfer(slope, 0.5f32.mul_add(-slope, 0.5))
        }
        Filter::Grayscale(amount) => toward(rows(LUMA), unit(amount)),
        Filter::Sepia(amount) => toward(SEPIA, unit(amount)),
        Filter::Saturate(amount) => saturate(amount.max(0.0)),
        Filter::HueRotate(degrees) => hue_rotate(degrees.to_radians()),
        Filter::Invert(amount) => {
            let amount = unit(amount);
            transfer(2.0f32.mul_add(-amount, 1.0), amount)
        }
        #[cfg(feature = "kittest")]
        Filter::Opacity(amount) => {
            let mut matrix = rgb(IDENTITY);
            matrix[3][3] = unit(amount);
            matrix
        }
    })
}

pub(crate) fn apply(matrix: &ColorMatrix, color: [f32; 4]) -> [f32; 4] {
    matrix.map(|row| {
        let sum = row[..4]
            .iter()
            .zip(color)
            .fold(row[4], |sum, (weight, channel)| {
                weight.mul_add(channel, sum)
            });
        sum.clamp(0.0, 1.0)
    })
}

const fn rows(luma: [f32; 3]) -> [[f32; 3]; 3] {
    [luma, luma, luma]
}

fn cellwise(
    left: [[f32; 3]; 3],
    right: [[f32; 3]; 3],
    cell: impl Fn(f32, f32) -> f32,
) -> [[f32; 3]; 3] {
    let mut out = [[0.0; 3]; 3];
    for ((out_row, left_row), right_row) in out.iter_mut().zip(left).zip(right) {
        for ((slot, left_value), right_value) in out_row.iter_mut().zip(left_row).zip(right_row) {
            *slot = cell(left_value, right_value);
        }
    }
    out
}

fn toward(target: [[f32; 3]; 3], amount: f32) -> ColorMatrix {
    rgb(cellwise(IDENTITY, target, |identity, value| {
        (identity - value).mul_add(1.0 - amount, value)
    }))
}

fn saturate(amount: f32) -> ColorMatrix {
    rgb(cellwise(IDENTITY, rows(SATURATE_LUMA), |identity, luma| {
        (identity - luma).mul_add(amount, luma)
    }))
}

fn hue_rotate(radians: f32) -> ColorMatrix {
    let (sin, cos) = radians.sin_cos();
    let base = cellwise(HUE_COS, rows(SATURATE_LUMA), |weight, luma| {
        weight.mul_add(cos, luma)
    });
    rgb(cellwise(HUE_SIN, base, |weight, value| {
        weight.mul_add(sin, value)
    }))
}

fn transfer(slope: f32, intercept: f32) -> ColorMatrix {
    let mut matrix = rgb(IDENTITY.map(|row| row.map(|value| value * slope)));
    for row in &mut matrix[..3] {
        row[4] = intercept;
    }
    matrix
}

fn rgb(block: [[f32; 3]; 3]) -> ColorMatrix {
    let mut matrix = [[0.0; 5]; 4];
    for (row, values) in matrix.iter_mut().zip(block) {
        row[..3].copy_from_slice(&values);
    }
    matrix[3][3] = 1.0;
    matrix
}
