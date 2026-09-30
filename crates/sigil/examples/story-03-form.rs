use sigil::prelude::*;

fn main() -> AppResult {
    run("Form", form)
}

pub(crate) fn form() -> impl IntoNode {
    let name = signal(String::new());
    let email = signal(String::new());
    let thanks = signal(String::new());
    let email_error = move || {
        let value = email.get();
        (!value.is_empty() && !value.contains('@')).then(|| "Enter an email".to_string())
    };
    let incomplete = move || name.get().trim().is_empty() || !email.get().contains('@');
    column()
        .child(field("Name", text_field(name).placeholder("Name")))
        .child(field("Email", text_field(email).placeholder("Email")).error(email_error))
        .child(
            button("Submit")
                .disabled(incomplete)
                .activate(move || thanks.set(format!("Thanks, {}", name.get()))),
        )
        .child(text(thanks))
}
