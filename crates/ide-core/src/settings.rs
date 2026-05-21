//! Settings — layered key/value store with TOML hot-reload.
//!
//! Three-layer precedence (highest first):
//!   1. **workspace** — `.sigil-ide/settings.toml` under the workspace root
//!   2. **user** — platform user-config dir (`~/.config/sigil-ide/settings.toml` on Unix,
//!      `%APPDATA%\sigil-ide\settings.toml` on Windows)
//!   3. **default** — compiled-in default registered via [`Schema::register`]
//!
//! Unknown keys and type-mismatched values do **not** block load — they
//! emit [`Diagnostic`] entries returned alongside the [`Store`].
//!
//! Hot-reload is opt-in: filesystem watching is **not** part of this
//! primitive. Callers wire [`Store::reload`] to whatever signal they
//! prefer (idle tick, [`crate::workspace`] watcher, manual command).
//! [`Store::watch`] callbacks fire only when a key's *resolved* value
//! changes.
//!
//! # Example
//!
//! ```ignore
//! use ide_core::settings::{Schema, Store};
//!
//! let mut schema = Schema::new();
//! schema.register("editor.tab_width", 4_i64);
//! schema.register("editor.word_wrap", false);
//!
//! let (store, _diags) = Store::open(&workspace_root, schema)?;
//! let tab_width: i64 = store.get("editor.tab_width").unwrap_or(4);
//! ```

mod diagnostic;
mod schema;
mod store;
mod value;
mod watch;

pub use diagnostic::Diagnostic;
pub use schema::Schema;
pub use store::Store;
pub use value::SettingValue;
pub use watch::Subscription;

#[cfg(test)]
mod tests;
