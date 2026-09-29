use sigil::prelude::*;

fn main() -> AppResult {
    run("Hello", hello)
}

pub(crate) fn hello() -> impl IntoNode {
    text("Hello, world")
}
