#[path = "story_10_layer2_parity/button.rs"]
mod button;
#[path = "story_10_layer2_parity/slider.rs"]
mod slider;
#[path = "story_10_layer2_parity/text_field.rs"]
mod text_field;

#[allow(unused_imports, reason = "a test crate that includes this file uses only the constructors")]
pub use button::{CustomButtonBuilder, custom_button};
#[allow(unused_imports, reason = "a test crate that includes this file uses only the constructors")]
pub use slider::{CustomSliderBuilder, custom_slider};
#[allow(unused_imports, reason = "a test crate that includes this file uses only the constructors")]
pub use text_field::{CustomTextFieldBuilder, custom_text_field};

use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::run("Layer 2 Parity", app)
}

/// Standalone Layer 2 parity screen exercising `custom_button`, `custom_slider`, and `custom_text_field`.
#[must_use]
pub fn app() -> impl IntoNode {
    let clicks = signal(0_u32);
    let volume = signal(25.0_f32);
    let name = signal(String::new());
    let submitted = signal(String::from("none"));
    column()
        .padding(Space::Lg)
        .gap(Space::Md)
        .background(Token::Surface)
        .child(text("Layer 2 Widget Parity").text_role(TextRole::Title))
        .child(
            custom_button("Save")
                .primary()
                .activate(move || clicks.update(|n| *n += 1)),
        )
        .child(text(move || format!("Clicks: {}", clicks.get())))
        .child(
            custom_slider(volume, 0.0..=100.0)
                .step(5.0)
                .length(240.0)
                .label("Volume"),
        )
        .child(text(move || format!("Volume: {:.0}", volume.get())))
        .child(
            custom_text_field(name)
                .placeholder("Enter name")
                .width(240.0)
                .submit(move |v| submitted.set(v.to_owned())),
        )
        .child(text(move || format!("Submitted: {}", submitted.get())))
}
