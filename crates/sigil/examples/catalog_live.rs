#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

#[path = "visual_catalog/builders.rs"]
mod builders;
#[path = "visual_catalog/chrome.rs"]
mod chrome;
#[path = "visual_catalog/content.rs"]
mod content;
#[path = "visual_catalog/controls.rs"]
mod controls;
#[path = "visual_catalog/data.rs"]
mod data;
#[path = "visual_catalog/inputs.rs"]
mod inputs;
#[path = "visual_catalog/overlays.rs"]
mod overlays;
#[path = "visual_catalog/plumbing.rs"]
mod plumbing;
#[path = "visual_catalog/spec.rs"]
mod spec;

use std::cell::RefCell;

use sigil::prelude::*;
use sigil::tasks::on_message;
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

use spec::State;

thread_local! {
    static PICKS: RefCell<Option<UnboundedSender<Pick>>> = const { RefCell::new(None) };
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Pick {
    build: String,
    state: State,
    dark: bool,
    nonce: u32,
}

impl Pick {
    fn parse(query: &str) -> Self {
        let mut pick = Self {
            build: String::from("button"),
            state: State::Default,
            dark: false,
            nonce: 0,
        };
        for pair in query.trim_start_matches('?').split('&') {
            let Some((key, value)) = pair.split_once('=') else {
                continue;
            };
            match key {
                "build" => value.clone_into(&mut pick.build),
                "state" => {
                    pick.state = serde_json::from_value(serde_json::Value::from(value))
                        .unwrap_or(State::Default);
                }
                "theme" => pick.dark = value == "dark",
                "n" => pick.nonce = value.parse().unwrap_or(0),
                _ => {}
            }
        }
        pick
    }

    fn theme(&self) -> Theme {
        if self.dark {
            Theme::dark()
        } else {
            Theme::light()
        }
    }
}

fn widget(pick: &Pick) -> NodeBuilder {
    builders::builder(&pick.build).map_or_else(
        || spec::node(text(format!("No builder {}", pick.build))),
        |build| build(pick.state).layout(LayoutStyle::default().align_items(AlignItems::Center)),
    )
}

fn sandbox(first: Pick) -> impl IntoNode {
    let (sender, receiver) = unbounded_channel();
    PICKS.with(|picks| *picks.borrow_mut() = Some(sender));
    set_theme(first.theme());
    let picks = signal(vec![first]);
    on_message(receiver, move |next: Pick| {
        set_theme(next.theme());
        picks.set(vec![next]);
    });
    column()
        .layout(
            LayoutStyle::flex_column()
                .fill_width()
                .fill_height()
                .padding(24.0),
        )
        .bg(Token::Surface)
        .child(for_each(
            picks,
            |pick| (pick.build.clone(), pick.state, pick.nonce),
            widget,
        ))
}

#[cfg(target_arch = "wasm32")]
pub mod web {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(inline_js = "export function query() { return location.search; }")]
    extern "C" {
        pub(crate) fn query() -> String;
    }

    #[wasm_bindgen]
    pub fn select(query: &str) {
        super::PICKS.with(|picks| {
            if let Some(sender) = picks.borrow().as_ref() {
                drop(sender.send(super::Pick::parse(query)));
            }
        });
    }
}

#[cfg(target_arch = "wasm32")]
fn first_query() -> String {
    web::query()
}

#[cfg(not(target_arch = "wasm32"))]
fn first_query() -> String {
    std::env::args().nth(1).unwrap_or_default()
}

fn main() -> sigil::AppResult<()> {
    let first = Pick::parse(&first_query());
    sigil::run(
        AppConfig::new("Sigil - live")
            .size(480, 240)
            .canvas("sigil-live"),
        move || sandbox(first.clone()),
    )
}
