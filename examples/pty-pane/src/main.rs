//! One splitter pane is a PTY. The other is ordinary Sigil widgets.

#![allow(missing_docs)]

use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::launch(("Sigil - pty-pane", 800, 480), app_tree)
}

fn app_tree(cx: &mut Cx) -> NodeId {
    let session = match Session::spawn(&shell()) {
        Ok(session) => session,
        Err(err) => {
            let message = format!("pty: {err}");
            return cx.add(text(message));
        }
    };
    let lines = session.lines;
    cx.add(
        splitter(SplitterOrientation::Vertical)
            .width(800.0)
            .height(480.0)
            .pane(terminal(lines, move |input| {
                drop(session.write(input.as_bytes()));
            }))
            .grow(1.0)
            .pane(
                container()
                    .class("p-4 bg-surface")
                    .child(text("Widgets").class("text-sm"))
                    .child(button("Ping").activate(|| {})),
            )
            .initial(280.0),
    )
}

fn shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| String::from("/bin/zsh"))
}
