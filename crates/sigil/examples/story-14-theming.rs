use sigil::app::system_backdrop;
use sigil::prelude::*;
use sigil::style::Backdrop;

fn main() -> sigil::AppResult<()> {
    sigil::run("Theming & Subtrees", app)
}

const BRAND_BADGE: Token = Token::Custom("brand-badge");

#[derive(Clone, Copy, PartialEq, Eq)]
enum BackdropChoice {
    System,
    Light,
    Dark,
}

impl BackdropChoice {
    const fn label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Light => "Light",
            Self::Dark => "Dark",
        }
    }
}

fn active_backdrop(choice: BackdropChoice) -> Backdrop {
    match choice {
        BackdropChoice::System => system_backdrop().get().unwrap_or(Backdrop::Light),
        BackdropChoice::Light => Backdrop::Light,
        BackdropChoice::Dark => Backdrop::Dark,
    }
}

const fn backdrop_label(backdrop: Option<Backdrop>) -> &'static str {
    match backdrop {
        Some(Backdrop::Dark) => "Dark",
        Some(Backdrop::Light) => "Light",
        None => "None",
    }
}

fn brand_theme(backdrop: Backdrop) -> Theme {
    let (accent, accent_hover, on_accent, badge) = match backdrop {
        Backdrop::Light => (
            Color::rgb(109, 40, 217),
            Color::rgb(91, 33, 182),
            Color::WHITE,
            Color::rgb(15, 118, 110),
        ),
        Backdrop::Dark => (
            Color::rgb(167, 139, 250),
            Color::rgb(196, 181, 253),
            Color::rgb(15, 23, 42),
            Color::rgb(45, 212, 191),
        ),
    };
    Theme::for_backdrop(backdrop)
        .token(Token::Accent, accent)
        .token(Token::AccentHover, accent_hover)
        .token(Token::OnAccent, on_accent)
        .token(BRAND_BADGE, badge)
        .control_height(ControlSize::Compact, 28.0)
        .radius(Radius::Md, 10.0)
}

/// Interactive Layer 1 and Layer 2 theming showcase using `Theme::for_backdrop`, `Token::Custom`, `Widget::theme`, and `Paint` 2D transforms.
#[must_use]
pub fn app() -> impl IntoNode {
    let choice = signal(BackdropChoice::System);
    let clicks = signal(0_u32);

    column()
        .theme(move || Theme::for_backdrop(active_backdrop(choice.get())))
        .padding(Space::Md)
        .gap(Space::Sm)
        .background(Token::Surface)
        .child(text("Theming & Subtrees").text_role(TextRole::Title))
        .child(backdrop_controls(choice))
        .child(status_line(choice))
        .child(preview_card(choice, clicks))
}

fn backdrop_controls(choice: Signal<BackdropChoice>) -> NodeBuilder {
    row()
        .gap(Space::Sm)
        .align_items(AlignItems::Center)
        .child(mode_button("System", BackdropChoice::System, choice))
        .child(mode_button("Light", BackdropChoice::Light, choice))
        .child(mode_button("Dark", BackdropChoice::Dark, choice))
}

fn mode_button(
    label: &'static str,
    target: BackdropChoice,
    choice: Signal<BackdropChoice>,
) -> impl IntoNode {
    button(label)
        .size(ControlSize::Compact)
        .activate(move || choice.set(target))
}

fn status_line(choice: Signal<BackdropChoice>) -> NodeBuilder {
    row()
        .gap(Space::Md)
        .child(text(move || format!("Mode: {}", choice.get().label())).nowrap())
        .child(
            text(move || format!("System: {}", backdrop_label(system_backdrop().get()))).nowrap(),
        )
}

fn preview_card(choice: Signal<BackdropChoice>, clicks: Signal<u32>) -> impl IntoNode {
    card(
        column()
            .gap(Space::Sm)
            .child(
                row()
                    .gap(Space::Sm)
                    .align_items(AlignItems::Center)
                    .child(brand_badge())
                    .child(
                        button("Accent action")
                            .primary()
                            .size(ControlSize::Compact)
                            .activate(move || clicks.update(|count| *count += 1)),
                    ),
            )
            .child(text(move || format!("Action clicks: {}", clicks.get())).nowrap()),
    )
    .title("Subtree preview")
    .theme(move || brand_theme(active_backdrop(choice.get())))
}

fn brand_badge() -> NodeBuilder {
    row()
        .padding_x(Space::Md)
        .align_items(AlignItems::Center)
        .radius(Radius::Md)
        .background(BRAND_BADGE)
        .scale(1.0)
        .translate_x(0.0)
        .hover(|s| s.scale(1.04))
        .child(text("BRAND").text_color(Token::OnAccent).nowrap())
}
