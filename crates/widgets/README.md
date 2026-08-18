# sigil-widgets

First-party Sigil widget builders. No window, no GPU.

Depend on this crate to reuse `chart()`, `progress()`, `table()`, and the rest
without linking `wgpu` or `winit`. Desktop apps keep `use sigil::prelude::*`.
