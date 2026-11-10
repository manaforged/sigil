use sigil::prelude::*;
use sigil::reactive::Owner;

fn main() -> sigil::AppResult<()> {
    Owner::scope("app", || {
        let name = signal(String::new());
        let updates = signal(false);
        let saved = signal(0_u32);
        sigil::run(AppConfig::new("Profile"), move || {
            sigil::view!("examples/view-form.sigil")
                .signal("name", name)
                .flag("updates", updates)
                .action("save", move || saved.update(|count| *count += 1))
                .action("reset", move || name.set(String::new()))
                .slot("status", move || {
                    text(move || format!("Saved {} times", saved.get()))
                        .text_role(TextRole::Caption)
                })
        })
    })
}
