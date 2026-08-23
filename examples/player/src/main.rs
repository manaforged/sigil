//! Player chrome from catalog names. Bind `Player` signals.
//!
//! Pass a file to play: `cargo run -p player -- /path/to/track`.

#![allow(missing_docs)]

use std::path::PathBuf;

use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::launch(("Sigil - player", 480, 240), app_tree)
}

fn app_tree(cx: &mut Cx) -> NodeId {
    let player = Player::new();
    let playing = player.playing;
    let progress = player.progress;
    let status = signal(String::from("pass a file path to play"));
    if let Some(path) = std::env::args().nth(1).map(PathBuf::from) {
        match player.play_path(&path) {
            Ok(()) => status.set(path.display().to_string()),
            Err(err) => status.set(err.to_string()),
        }
    }
    cx.add(
        container()
            .layout(LayoutStyle::flex_row().fixed_size(480.0, 240.0))
            .class("bg-canvas")
            .child(sidebar(
                container()
                    .class("p-4")
                    .child(avatar("SG"))
                    .child(text("Library").class("text-sm")),
            ))
            .child(
                container()
                    .layout(LayoutStyle::flex_column().gap(12.0).fill())
                    .class("p-4")
                    .child(card(text("Track").class("text-sm")))
                    .child(text(status).class("text-sm"))
                    .child(
                        transport(playing, progress, {
                            let player = player.clone();
                            move || player.toggle()
                        })
                        .seek(move |ratio| drop(player.seek(ratio))),
                    ),
            ),
    )
}
