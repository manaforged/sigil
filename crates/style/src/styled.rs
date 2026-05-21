//! G6 `Styled` trait — unified fluent surface across `NodeBuilder` and
//! every widget builder. See DR-049 for the locked design.
//!
//! The trait is split into per-category super-traits, each in its own
//! file, all anchored on a single [`StyleAccess`] base:
//!
//! - [`access`] — [`StyleAccess`], the single `fn style_mut(&mut self)` hook
//!   every implementor provides.
//! - [`spacing`] — [`Spacing`]: padding, margin, gap.
//! - [`sizing`] — [`Sizing`]: width, height, min/max.
//! - [`flex`] — [`Flex`]: display, flex direction, alignment, grow/shrink.
//! - [`fill`] — [`Fill`]: background and text colour.
//! - [`typography`] — [`Typography`]: font size, weight, family, line height.
//! - [`geometry`] — [`Geometry`]: border radius.
//! - [`visual`] — [`Visual`]: opacity, overflow.
//! - [`transitions`] — [`Transitions`]: transition preset + duration.
//! - [`state_variants`] — [`StateVariants`]: hover/focus/pressed/disabled
//!   (gated on [`StateHost`]).
//! - [`umbrella`] — [`Styled`], the all-of-the-above bound for downstream
//!   "anything styleable" call sites.
//! - [`color`] — [`ColorOrToken`] polymorphic colour input.
//! - [`state`] — [`StateStyleBuilder`] + [`StateHost`] marker.
//! - [`node`] — [`NodeStyledBuilder`] + [`Styleable`] adapter.

pub(crate) mod access;
pub(crate) mod color;
pub(crate) mod fill;
pub(crate) mod flex;
pub(crate) mod geometry;
pub(crate) mod node;
pub(crate) mod sizing;
pub(crate) mod spacing;
pub(crate) mod state;
pub(crate) mod state_variants;
pub(crate) mod transitions;
pub(crate) mod typography;
pub(crate) mod umbrella;
pub(crate) mod visual;

pub use access::StyleAccess;
pub use color::ColorOrToken;
pub use fill::Fill;
pub use flex::Flex;
pub use geometry::Geometry;
pub use node::{NodeStyledBuilder, Styleable};
pub use sizing::Sizing;
pub use spacing::Spacing;
pub use state::{StateHost, StateStyleBuilder};
pub use state_variants::StateVariants;
pub use transitions::Transitions;
pub use typography::Typography;
pub use umbrella::Styled;
pub use visual::Visual;
