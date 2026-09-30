#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;
use sigil::widgets::ScrollBuilder;

const SECTIONS: &[&str] = &["Account", "Appearance", "Notifications"];
const LANGUAGES: &[&str] = &["English", "Deutsch", "Français", "日本語"];

#[derive(Clone, Copy)]
struct Prefs {
    name: Signal<String>,
    email: Signal<String>,
    language: Signal<usize>,
    dark: Signal<bool>,
    compact: Signal<bool>,
    scale: Signal<f32>,
    notify_email: Signal<bool>,
    notify_push: Signal<bool>,
    volume: Signal<f32>,
}

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("Preferences").size(880, 600).window(), build)
}

pub(crate) fn build() -> impl IntoNode {
    let page = signal(0_usize);
    let queue = ToastQueue::new();
    let prefs = Prefs {
        name: signal(String::from("Ada Lovelace")),
        email: signal(String::from("ada@example")),
        language: signal(0),
        dark: signal(false),
        compact: signal(false),
        scale: signal(100.0),
        notify_email: signal(true),
        notify_push: signal(false),
        volume: signal(60.0),
    };
    effect(move || {
        set_theme(if prefs.dark.get() {
            Theme::dark()
        } else {
            Theme::light()
        });
    });
    container()
        .role(Role::Main)
        .layout(LayoutStyle::flex_row().fill())
        .bg(Token::Surface)
        .child(sidebar(nav(page)))
        .child(
            container()
                .layout(LayoutStyle::flex_column().fill())
                .child(content(page, prefs))
                .child(footer(prefs, queue)),
        )
        .child(toast_queue(queue))
}

fn name_error(name: &str) -> Option<String> {
    if name.trim().is_empty() {
        Some(String::from("Name is required."))
    } else if name.chars().count() > 40 {
        Some(String::from("Name must be 40 characters or fewer."))
    } else {
        None
    }
}

fn email_error(email: &str) -> Option<String> {
    match email.split_once('@') {
        Some((user, host)) if !user.is_empty() && host.contains('.') => None,
        _ => Some(String::from("Enter an address like name@example.com.")),
    }
}

fn nav(page: Signal<usize>) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().fill_width())
        .child(section_label("Settings"))
        .children_reactive(move || {
            let current = page.get();
            SECTIONS
                .iter()
                .enumerate()
                .map(|(index, name)| {
                    list_row(index == current, *name, "")
                        .focusable(true)
                        .pointer_up(move |_| page.set(index))
                        .key(Key::Enter, move |_| page.set(index))
                        .key(LogicalKey::character(" "), move |_| page.set(index))
                })
                .collect::<Vec<_>>()
        })
}

fn content(page: Signal<usize>, prefs: Prefs) -> ScrollBuilder {
    scroll(
        container()
            .layout(
                LayoutStyle::flex_column()
                    .fill_width()
                    .padding(em(1.5))
                    .gap(rows(1.0)),
            )
            .children_reactive(move || {
                vec![match page.get() {
                    1 => appearance(prefs),
                    2 => notifications(prefs),
                    _ => account(prefs),
                }]
            }),
    )
    .fill()
}

fn page_layout() -> LayoutStyle {
    LayoutStyle::flex_column().fill_width().gap(rows(0.75))
}

fn half(child: impl IntoNode + 'static) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_column()
                .flex_basis(cols(30.0))
                .min_width(cols(24.0))
                .grow(1.0),
        )
        .child(child)
}

fn account(prefs: Prefs) -> NodeBuilder {
    let languages = LANGUAGES.iter().map(|s| String::from(*s)).collect();
    container()
        .layout(page_layout())
        .child(text("Account").font_size(18))
        .child(
            container()
                .layout(
                    LayoutStyle::flex_row()
                        .fill_width()
                        .gap(cols(2.0))
                        .flex_wrap(FlexWrap::Wrap),
                )
                .child(half(
                    field(
                        "Display name",
                        text_field(prefs.name).placeholder("Your name"),
                    )
                    .error(move || name_error(&prefs.name.get())),
                ))
                .child(half(
                    field(
                        "Email",
                        text_field(prefs.email).placeholder("name@example.com"),
                    )
                    .error(move || email_error(&prefs.email.get())),
                )),
        )
        .child(field(
            "Language",
            dropdown(prefs.language, languages).label("Language"),
        ))
}

fn appearance(prefs: Prefs) -> NodeBuilder {
    let scale_text = derived(move || format!("Interface scale: {:.0}%", prefs.scale.get()));
    container()
        .layout(page_layout())
        .child(text("Appearance").font_size(18))
        .child(toggle(prefs.dark).label("Dark theme"))
        .child(checkbox(prefs.compact).label("Compact rows"))
        .child(field(
            scale_text,
            slider(prefs.scale, 75.0..=150.0).step(5.0),
        ))
}

fn notifications(prefs: Prefs) -> NodeBuilder {
    let volume_text = derived(move || format!("Alert volume: {:.0}", prefs.volume.get()));
    container()
        .layout(page_layout())
        .child(text("Notifications").font_size(18))
        .child(toggle(prefs.notify_email).label("Email digests"))
        .child(toggle(prefs.notify_push).label("Push notifications"))
        .child(field(
            volume_text,
            slider(prefs.volume, 0.0..=100.0).step(1.0),
        ))
}

fn footer(prefs: Prefs, queue: ToastQueue) -> NodeBuilder {
    let invalid = derived(move || {
        name_error(&prefs.name.get()).is_some() || email_error(&prefs.email.get()).is_some()
    });
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .padding(em(0.75))
                .gap(cols(1.0))
                .align_items(AlignItems::Center),
        )
        .bg(Token::SurfaceHover)
        .child(spacer())
        .child(
            button("Save")
                .primary()
                .disabled(invalid)
                .activate(move || {
                    queue.success(format!("Saved preferences for {}.", prefs.name.peek()));
                }),
        )
}
