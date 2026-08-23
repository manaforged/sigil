# sigil-widgets

First-party Sigil widget builders. No window, no GPU.

The catalog includes `button`, `icon_button`, `text_field`, `text_edit`,
`table`, `virtual_list`, `chart`, `progress`, `card`, `avatar`,
`sidebar`, and the rest. Depend on this crate to reuse them without
linking `wgpu` or `winit`. Desktop apps keep `use sigil::prelude::*;`.
