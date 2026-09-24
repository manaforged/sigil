use std::fmt::{self, Write as _};
use std::path::Path;

use sigil::style::{Metrics, Radius, Rgba8, Size, Theme, Token, space};
use skrifa::instance::{LocationRef, Size as FontSize};
use skrifa::string::StringId;
use skrifa::{FontRef, MetadataProvider};

use super::capture::Look;

const FONTS: &[(&str, &[u8])] = &[
    (
        "FiraSans-Regular.ttf",
        include_bytes!("../../assets/text/fonts/FiraSans-Regular.ttf"),
    ),
    (
        "FiraSans-Medium.ttf",
        include_bytes!("../../assets/text/fonts/FiraSans-Medium.ttf"),
    ),
    (
        "FiraSans-SemiBold.ttf",
        include_bytes!("../../assets/text/fonts/FiraSans-SemiBold.ttf"),
    ),
    (
        "CascadiaMono-Regular.ttf",
        include_bytes!("../../assets/text/fonts/CascadiaMono-Regular.ttf"),
    ),
];
const LICENCE: (&str, &[u8]) = (
    "LICENSE-OFL.txt",
    include_bytes!("../../assets/text/fonts/LICENSE-OFL.txt"),
);
const RADII: [(&str, Radius); 4] = [
    ("sm", Radius::Sm),
    ("md", Radius::Md),
    ("lg", Radius::Lg),
    ("full", Radius::Full),
];
const SPACE_STEPS: u8 = 16;

struct Face {
    file: &'static str,
    family: String,
    weight: f32,
    mono: bool,
}

pub(crate) fn write_fonts(out: &Path) -> Result<(), String> {
    let dir = out.join("fonts");
    std::fs::create_dir_all(&dir).map_err(|err| format!("create fonts: {err}"))?;
    for (name, bytes) in FONTS.iter().chain([&LICENCE]) {
        std::fs::write(dir.join(name), bytes).map_err(|err| format!("write {name}: {err}"))?;
    }
    Ok(())
}

pub(crate) fn css() -> Result<String, String> {
    let faces = FONTS
        .iter()
        .map(|(file, bytes)| face(file, bytes))
        .collect::<Result<Vec<_>, _>>()?;
    sheet(&faces).map_err(|err| format!("tokens.css: {err}"))
}

fn sheet(faces: &[Face]) -> Result<String, fmt::Error> {
    let mut out = String::new();
    for face in faces {
        font_face(&mut out, face)?;
    }
    out.push_str(":root {\n");
    colors(&mut out, &Look::Light.theme(), "  ")?;
    metrics(&mut out, faces)?;
    out.push_str("}\n\n[data-theme=dark] {\n");
    colors(&mut out, &Look::Dark.theme(), "  ")?;
    out.push_str("}\n\n@media (prefers-color-scheme: dark) {\n  :root:not([data-theme=light]) {\n");
    colors(&mut out, &Look::Dark.theme(), "    ")?;
    out.push_str("  }\n}\n");
    Ok(out)
}

fn face(file: &'static str, bytes: &[u8]) -> Result<Face, String> {
    let font = FontRef::new(bytes).map_err(|err| format!("parse {file}: {err}"))?;
    let family = [StringId::TYPOGRAPHIC_FAMILY_NAME, StringId::FAMILY_NAME]
        .into_iter()
        .find_map(|id| font.localized_strings(id).english_or_first())
        .map(|name| name.to_string())
        .ok_or_else(|| format!("{file}: no family name"))?;
    Ok(Face {
        file,
        family,
        weight: font.attributes().weight.value(),
        mono: font
            .metrics(FontSize::unscaled(), LocationRef::default())
            .is_monospace,
    })
}

fn font_face(out: &mut String, face: &Face) -> fmt::Result {
    writeln!(
        out,
        "@font-face {{\n  font-family: \"{}\";\n  src: url(\"fonts/{}\") format(\"truetype\");\n  font-weight: {};\n  font-style: normal;\n  font-display: swap;\n}}\n",
        face.family, face.file, face.weight
    )
}

fn colors(out: &mut String, theme: &Theme, indent: &str) -> fmt::Result {
    for token in Token::ALL {
        let value = hex(theme.color(*token));
        writeln!(out, "{indent}--sg-{}: {value};", token.name())?;
    }
    Ok(())
}

pub(crate) fn hex(color: Rgba8) -> String {
    let rgb = format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b);
    if color.a == u8::MAX {
        rgb
    } else {
        format!("{rgb}{:02x}", color.a)
    }
}

fn metrics(out: &mut String, faces: &[Face]) -> fmt::Result {
    let window = Metrics::window();
    for (name, radius) in RADII {
        writeln!(out, "  --sg-radius-{name}: {}px;", window.radius(radius).0)?;
    }
    for step in 0..=SPACE_STEPS {
        writeln!(out, "  --sg-space-{step}: {}px;", space(step))?;
    }
    writeln!(
        out,
        "  --sg-control-height: {}px;\n  --sg-focus-ring-width: {}px;\n  --sg-focus-ring-offset: {}px;",
        window.control_height(Size::Default),
        window.ring_width(),
        window.ring_offset()
    )?;
    let family = |mono: bool| {
        faces
            .iter()
            .find(|face| face.mono == mono)
            .map_or_else(String::new, |face| format!("\"{}\", ", face.family))
    };
    writeln!(
        out,
        "  --sg-font-sans: {}system-ui, sans-serif;\n  --sg-font-mono: {}ui-monospace, monospace;",
        family(false),
        family(true)
    )
}
