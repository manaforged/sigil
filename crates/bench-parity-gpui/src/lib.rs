#![allow(clippy::module_name_repetitions)]

use clap as _;
use serde as _;
use serde_json as _;
use time as _;

mod gpui_runner;

pub use gpui_runner::GpuiRunner;
