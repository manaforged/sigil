//! `ledger-gpui [out.json]` — opens the GPUI Ledger, replays the shared
//! script, writes a record-v1, and quits.

use data_table as _;

use std::path::PathBuf;

use bench_gpui::Ledger;
use bench_spec::ledger::{ROWS, WINDOW_PX};
use gpui::{App, AppContext as _, Application, Bounds, WindowBounds, WindowOptions, px, size};

fn main() {
    let out = std::env::args()
        .nth(1)
        .map_or_else(|| PathBuf::from("ledger-gpui.json"), PathBuf::from);
    Application::new().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(WINDOW_PX.0), px(WINDOW_PX.1)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        };
        let opened = cx.open_window(options, |_, cx| {
            cx.new(|cx| Ledger::new(ROWS, out.clone(), cx))
        });
        if let Err(err) = opened {
            eprintln!("ledger-gpui: open window: {err}");
            cx.quit();
            return;
        }
        cx.activate(true);
    });
}
