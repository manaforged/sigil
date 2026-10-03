#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("Sigil - settings").size(880, 600), app)
}

fn navigation() -> sigil::widgets::SidebarBuilder {
    sidebar(
        container()
            .layout(LayoutStyle::flex_column())
            .child(section_label("Settings"))
            .child(list_row(true, "General", ""))
            .child(list_row(false, "Appearance", ""))
            .child(list_row(false, "Notifications", ""))
            .child(list_row(false, "Privacy", "")),
    )
}

fn profile_section() -> NodeBuilder {
    let name = signal(String::from("Ada Lovelace"));
    let language = signal(0_usize);
    container()
        .layout(LayoutStyle::flex_column().gap(rows(1.0)))
        .child(section_label("Profile"))
        .child(field("Display name", text_field(name).placeholder("Name")))
        .child(field(
            "Language",
            dropdown(
                language,
                vec!["English".into(), "Deutsch".into(), "Français".into()],
            ),
        ))
}

fn appearance_section() -> NodeBuilder {
    let theme = signal(String::from("system"));
    let scale = signal(100.0_f32);
    container()
        .layout(LayoutStyle::flex_column().gap(rows(1.0)))
        .child(section_label("Appearance"))
        .child(field(
            "Theme",
            radio_group(theme)
                .label("Theme")
                .option("light", "Light")
                .option("dark", "Dark")
                .option("system", "System"),
        ))
        .child(field(
            "Interface scale",
            slider(scale, 75.0..=150.0)
                .step(25.0)
                .label("Interface scale"),
        ))
}

fn notifications_section() -> NodeBuilder {
    let desktop = signal(true);
    let digest = signal(false);
    container()
        .layout(LayoutStyle::flex_column().gap(rows(1.0)))
        .child(section_label("Notifications"))
        .child(field(
            "Desktop alerts",
            toggle(desktop).label("Show desktop alerts"),
        ))
        .child(field(
            "Email",
            checkbox(digest).label("Send a weekly digest"),
        ))
}

fn footer() -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .gap(cols(1.0))
                .padding(cols(2.0))
                .justify_content(JustifyContent::End),
        )
        .child(button("Cancel"))
        .child(button("Save").primary())
}

fn content() -> NodeBuilder {
    container()
        .role(Role::Main)
        .layout(LayoutStyle::flex_column().fill())
        .child(
            scroll(
                container()
                    .layout(LayoutStyle::flex_column().gap(rows(2.0)).padding(cols(3.0)))
                    .child(profile_section())
                    .child(appearance_section())
                    .child(notifications_section()),
            )
            .fill(),
        )
        .child(divider_horizontal())
        .child(footer())
}

pub(crate) fn app() -> impl IntoNode {
    container()
        .layout(LayoutStyle::flex_row().fill())
        .child(navigation())
        .child(content())
}
