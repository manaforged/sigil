//! Buffer — rope-backed text buffer + supporting types.
//!
//! W3.5a (I-T2) landed the rope: an owned, persistent B-tree rope that
//! tracks bytes / chars / lines / UTF-16 code units per node. The rope
//! is the substrate for I-T3 history (cheap `Arc` snapshots) and I-T4
//! multi-cursor edit batching.
//!
//! Why owned, not `ropey`: see DR-040 (own what we'd ever replace). The
//! rope is zero-runtime-deps, and the node-level count caches let us
//! attach history + cursor batching at the tree level for the perf budget
//! needed at G12 scale.

pub mod coords;
pub mod cursor;
pub mod history;
pub mod rope;
pub mod rope_iter;
pub mod rope_node;

pub use coords::{ByteIdx, CharIdx, LineCol};
pub use cursor::{Cursor, CursorSet, Edit};
pub use history::History;
pub use rope::{Rope, RopeSlice};
pub use rope_iter::{RopeBytesIter, RopeCharsIter, RopeLinesIter};
