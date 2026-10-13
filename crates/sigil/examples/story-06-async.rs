use std::time::Duration;

use sigil::prelude::*;
use sigil::tasks::sleep;

fn main() -> AppResult {
    run("Async", loader)
}

async fn fetch() -> Result<String, String> {
    sleep(Duration::from_millis(500)).await;
    Ok("42 items".into())
}

pub(crate) fn loader() -> impl IntoNode {
    let load = task(fetch);
    column()
        .child(button("Load").activate(move || load.run()))
        .child(text(move || match load.status() {
            Status::Idle => String::new(),
            Status::Pending => "Loading…".into(),
            Status::Ok(items) => items,
            Status::Err(error) => format!("Error: {error}"),
            Status::Panicked(message) => format!("Crashed: {message}"),
        }))
}
