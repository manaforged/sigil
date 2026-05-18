//! Keymap — keystroke parser + chord-aware binding resolver.
//!
//! Three-layer TOML config (highest precedence first):
//!   1. **workspace** — `.sigil-ide/keymap.toml` under the workspace root
//!   2. **user** — platform user-config dir
//!      (`~/.config/sigil-ide/keymap.toml` on Unix,
//!      `%APPDATA%\sigil-ide\keymap.toml` on Windows)
//!   3. **default** — compiled-in [`Bindings`] registered via API
//!
//! # Primitive shape
//!
//! - [`Key::parse`] — textual `"ctrl-shift-f1"` → typed [`Key`]
//! - [`Bindings`] — flat binding set (action + optional context predicate)
//! - [`Keymap::open`] — load all three layers + flatten, returning
//!   non-fatal [`KeymapDiagnostic`] entries for malformed entries
//! - [`Resolver`] — stateful chord-aware lookup; emits [`Resolution`]
//!
//! # Example
//!
//! ```ignore
//! use std::borrow::Cow;
//! use sigil_ide_core::keymap::{Bindings, Key, Keymap, Resolver, Resolution, ContextSet};
//!
//! let mut defaults = Bindings::new();
//! defaults.insert(vec![Key::parse("ctrl-s").unwrap()], "buffer.save", []);
//!
//! let (keymap, _diags) = Keymap::open(workspace_root, defaults)?;
//! let mut resolver = Resolver::new();
//! let ctx = ContextSet::from_iter(["editor"]);
//!
//! let res = resolver.feed(keymap.bindings(), &ctx, Key::parse("ctrl-s").unwrap());
//! assert_eq!(res, Resolution::Action(Cow::Borrowed("buffer.save")));
//! ```
//!
//! # Followups
//!
//! - Full `when`-expression evaluator (the first cut is a string-set
//!   `superset` match).
//! - Hot-reload via the [`crate::workspace`] fs-watch primitive.
//! - Public reverse lookup ("which key triggers this action?") — the
//!   `_reverse_index` helper exists in [`resolver`] but is `doc(hidden)`
//!   until it gets a real surface.

mod config;
mod key;
mod parse;
mod resolver;

pub use config::{Keymap, KeymapDiagnostic};
pub use key::{Key, KeyCode, Modifier, Modifiers, NamedKey, ParseError};
pub use resolver::{Bindings, ContextSet, Resolution, Resolver};

#[cfg(test)]
mod tests;
