use std::sync::LazyLock;

use merman::svg::{
    DeterministicTextMeasurer, HostTheme, HostThemeAppearance, MeasurementProfileId, Presentation,
    SvgPipeline, TextMeasurementPolicy, TextMeasurementProfile, TextMeasurementProfileIdentity,
    TextStyle, ThemeRole,
};
use merman::{
    Engine, OperationControl, RenderOutput, RenderRequest, Renderer, SvgEnvironment, SvgRequest,
};

use crate::kernel::NodeBuilder;
use crate::style::palette::is_dark;
use crate::style::{Theme, Token};
use crate::text::Font;
use crate::text::shaper::{FaceStyle, Shaper, TextSegment};
use crate::widgets::chart::series_tokens;
use crate::widgets::svg_picture::{FONT_FAMILY, hex, picture};

const FONT_SIZE: &str = "16px";
const PROFILE: &str = "sigil-fira-sans";
const PROFILE_VERSION: &str = "1";
const BOLD: u16 = 700;
const REGULAR: u16 = 400;
const FALLBACK_EM: f64 = 0.55;

const ROLES: [(ThemeRole, Token); 20] = [
    (ThemeRole::Canvas, Token::Surface),
    (ThemeRole::Surface, Token::Accent3),
    (ThemeRole::SurfaceAlt, Token::Accent2),
    (ThemeRole::SurfaceMuted, Token::Gray2),
    (ThemeRole::Text, Token::OnSurface),
    (ThemeRole::SubtleText, Token::OnSurfaceMuted),
    (ThemeRole::Border, Token::Accent8),
    (ThemeRole::Line, Token::OnSurfaceMuted),
    (ThemeRole::EdgeLabelBackground, Token::SurfaceSubtle),
    (ThemeRole::ClusterBackground, Token::Gray2),
    (ThemeRole::ClusterBorder, Token::Border),
    (ThemeRole::NoteBackground, Token::Warn3),
    (ThemeRole::NoteBorder, Token::Warn8),
    (ThemeRole::ActorBackground, Token::Accent3),
    (ThemeRole::ActorBorder, Token::Accent8),
    (ThemeRole::ActivationBackground, Token::Accent4),
    (ThemeRole::ActivationBorder, Token::Accent8),
    (ThemeRole::Error, Token::Error9),
    (ThemeRole::Warning, Token::Warn9),
    (ThemeRole::Success, Token::Success9),
];

static FACES: LazyLock<Vec<(u16, Font)>> = LazyLock::new(|| {
    [
        (REGULAR, sigil_fonts::FIRA_SANS_REGULAR),
        (500, sigil_fonts::FIRA_SANS_MEDIUM),
        (600, sigil_fonts::FIRA_SANS_SEMIBOLD),
    ]
    .into_iter()
    .filter_map(|(weight, bytes)| Some((weight, Font::from_bytes(bytes.to_vec(), 0).ok()?)))
    .collect()
});

static MEASUREMENT: LazyLock<Option<TextMeasurementPolicy>> = LazyLock::new(measurement);

pub(super) fn figure(source: &str) -> Option<(NodeBuilder, (f32, f32))> {
    let owned = source.to_string();
    picture(source, move |theme| render(&owned, theme))
}

fn render(source: &str, theme: &Theme) -> Option<String> {
    let resolved = Presentation::new().with_theme(host_theme(theme)?).resolve();
    let renderer = Renderer::new().with_engine(resolved.materialize_engine(Engine::new()));
    let request = SvgRequest {
        environment: environment(),
        pipeline: Some(SvgPipeline::resvg_safe()),
        presentation: resolved.render_policy(),
        ..SvgRequest::default()
    };
    let output = renderer
        .render(RenderRequest::svg(source, OperationControl::new(), request))
        .ok()?;
    match output {
        RenderOutput::Svg(Some(svg)) => Some(svg.into_parts().0),
        _ => None,
    }
}

fn host_theme(theme: &Theme) -> Option<HostTheme> {
    let color = |token| hex(theme, token);
    let appearance = if is_dark(theme.surface) {
        HostThemeAppearance::Dark
    } else {
        HostThemeAppearance::Light
    };
    let mut host = HostTheme::new()
        .with_appearance(appearance)
        .try_with_font_family(FONT_FAMILY)
        .ok()?
        .try_with_font_size(FONT_SIZE)
        .ok()?;
    for (role, token) in ROLES {
        host = host.try_with_role(role, color(token)).ok()?;
    }
    host.try_with_series_palette(series_tokens().map(color))
        .ok()
}

fn environment() -> SvgEnvironment {
    let environment = SvgEnvironment::deterministic();
    match MEASUREMENT.as_ref() {
        Some(policy) => environment.with_text_measurement_policy(policy.clone()),
        None => environment,
    }
}

fn measurement() -> Option<TextMeasurementPolicy> {
    let profile = MeasurementProfileId::new(PROFILE).ok()?;
    let identity = TextMeasurementProfileIdentity::new(profile, PROFILE_VERSION).ok()?;
    let measurer = DeterministicTextMeasurer::default().with_width_callback(width);
    Some(TextMeasurementPolicy::uniform(TextMeasurementProfile::new(
        identity, measurer,
    )))
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "diagram font sizes and label lengths are far inside the f32 range"
)]
fn width(text: &str, style: &TextStyle) -> f64 {
    let weight = weight(style);
    let Some(font) = face(weight) else {
        return FALLBACK_EM * style.font_size * text.chars().count() as f64;
    };
    let face_style = FaceStyle {
        weight,
        ..FaceStyle::regular(style.font_size.max(1.0) as f32)
    };
    let run = Shaper::new(font).shape(&TextSegment::at_level(text, 0), &face_style);
    f64::from(run.width.max(0.0))
}

fn weight(style: &TextStyle) -> u16 {
    match style.font_weight.as_deref().map(str::trim) {
        Some("bold" | "bolder") => BOLD,
        Some(value) => value.parse().unwrap_or(REGULAR),
        None => REGULAR,
    }
}

fn face(weight: u16) -> Option<&'static Font> {
    FACES
        .iter()
        .rev()
        .find(|(face, _)| *face <= weight)
        .or_else(|| FACES.first())
        .map(|(_, font)| font)
}
