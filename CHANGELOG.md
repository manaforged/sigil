# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html). The `sigil-ui`, `sigil-fonts`, `sigil-sign`, and `sigil-kittest` crates share one version.

## [Unreleased]

### Added

- Canvas authors can build every scene payload without the `kittest` feature: `GradientStop::new`, `Gradient::{new, repeat}`, `Stroke::new`, `DrawGradient::new`, `DrawPattern::new`, `DrawDecoration::new`, `stroked` on the box payloads, `Shadow::{kind, reach}`, `Transform3D::{IDENTITY, then, translate, rotate_x, rotate_y, rotate_z, perspective, around}`, and `SceneBuilder::{draw_pattern, draw_decoration, draw_text_shadow, composite}`.
- `Canvas::redraw_on(source)` repaints a canvas when a signal, memo, or closure changes.
- `Render::font(family, bytes, face)` in `sigil-kittest` renders an offscreen image with a custom font.
- `DefaultFonts::set(family, bytes, face)` and a `serif` slot.
- `Paint::{backdrop_blur, backdrop_brightness, backdrop_saturate}` and the classes `backdrop-blur-*`, `backdrop-brightness-*`, and `backdrop-saturate-*` filter what is painted behind a node, inside its corners, like CSS `backdrop-filter`, on the wgpu and CPU renderers. The terminal host draws the node without the filter.
- `Property::{BackdropBlur, BackdropBrightness, BackdropSaturate}` animate the backdrop filter with `.drive` and keyframes.
- `LayerEffect::Backdrop`, and `Mask::new` and `LayerScope::layer_effect` without the `kittest` feature.
- `PointerEvent::velocity()` returns the pointer speed in logical pixels per second over its last moves; it is zero after a pause, so a held release does not fling.
- `Animator::fling(velocity)` coasts to rest, `Animator::fling_to(velocity, points)` springs to the point the fling would reach, and `Animator::velocity()` reads the current speed in value units per second.
- `Spring::duration(d).bounce(b)` builds a spring from a visual duration and a bounce, `Transition::spring(spring)` gives any spring to a style transition, `Keyframes::delay(d)` holds the start frame, and `Keyframes::fade()` fades in.
- `Paint::preserve_3d()` keeps the 3D transforms of children in the 3D space of the node, like CSS `transform-style: preserve-3d`, so a flip card is one rotating parent with two `backface_hidden()` faces. Opacity below 1, filters, clip paths, masks, blend modes and clipping overflow force flat, as in CSS.
- `Widget::enter(property)` and `Widget::exit(property)` animate a node from a start value when it mounts and to an end value before it is removed, on the timing of its `.transition`. A keyed row of `for_each` holds its space until the exit ends.
- `Property::{GradientAngle, GradientCenterX, GradientCenterY}` animate the angle and centre of a gradient background, and `GradientAxis::Angle(degrees)` sets any linear angle. `GradientAxis` is now `#[non_exhaustive]`; add a `_` arm to a match on it.
- `words(text, |index, word| ..)` lays text out as one node per word behind one accessible name, and `ticker(value)` rolls a number to each new value with `.format` and `.spring`. `Keyframes` and `Spring` are in the prelude.
- `animation::reduced_transparency()` and `AppConfig::reduced_transparency(bool)`. With the setting on, a backdrop filter is skipped and the node keeps its own fill. `animation::reduced_motion()` is now reactive, both settings are read again when the window gains focus, and `clock()` holds still under reduced motion.
- `Harness::step(by)` in `sigil-kittest` moves the test clock and stops there, so a test can read a running animation. `Harness::advance` still settles afterwards.
- `accessibility_hidden(bool)` on any node keeps it and its subtree out of the accessibility tree, like `aria-hidden`. The pieces of `words` and `ticker` use it, so a screen reader reads the text once. The index that `words` passes is now a `u32`, so `STAGGER * index` needs no cast.
- `Paint::shadow` takes `impl Into<Shadow>`, so a node can carry `Shadow::glow(color, blur, spread)`; `.shadow(Elevation::Lg)` still compiles. `Shadow::glow` now takes blur before spread, the order of `Shadow::new` and CSS `box-shadow`: swap the last two arguments of an existing call.

### Fixed

- `AppConfig::font(FontFamily::Serif, ..)` loads the face; it was dropped with a warning.
- Changing the transition of a running style animation keeps its value and velocity; it restarted from rest.

## [0.2.0] - 2026-11-12

First public release.

### Added

- `sigil::widgets::{Framed, WidgetFrame}` and unsealed `Paint`, `States`, and `Widget` traits with blanket implementations for `T: Framed`.
- Public `sigil::node` module, public `sigil::layout` module with `sigil::layout::LayoutStyle`, and `Widget::layout(self, style: LayoutStyle) -> Self` (replacing the `kittest`-only `Layout` trait).
- Public `sigil::text` module with `TextMeasure`, `CaretGeometry`, `HitPosition`, `TextAffinity`, `TextStyle`, `measure_text`, `col_count`, `col_at_byte`, and `byte_at_col`.
- `sigil::events::{DragPhase, A11yEdit}` and `PointerEvent::{prevent_default, default_prevented}`.
- `Widget::{track_hovered, track_focused, track_pressed, pointer_drag, a11y_edit, capture_text_style, ime_caret}` and `NodeBuilder::{busy, invalid}`.
- Public `TextState` editing methods, `Token::Selection` (`Token::COUNT` is now 95), `Transition::smooth`, `Theme::label`, `Metrics::track_size`, `clipboard::current`, `IconBuilder::on_solid`, and `From<&TextContent> for AccessibleName`.
- Public `sigil::scene` drawing primitives: `SceneCommand`, `Clip`, `Transform`, `DrawRect`, `DrawGradient`, `DrawPattern`, `DrawImage`, `DrawPath`, `DrawDecoration`, `LayerScope`, `CommandScene::{commands, iter, len, is_empty}` + `IntoIterator for &CommandScene`, `SceneBuilder::{new, fill_rect, draw_rect, draw_wash, draw_gradient, draw_path, stroke_line, draw_image, draw_shadow, draw_text, measure_text, clip, transform, finish}`, and `PathBuilder::build_path`.
- `KeyEvent::{prevent_default, default_prevented}`, `Widget::{offset, track_bounds, track_position, track_size, accessible_shortcut, drop_hover}`, and `sigil::events::DropEvent<T>` (`into_value`, `value`, `position`, `local_position`, `bounds`).
- Public `sigil::animation::{Animator, Spring, every_frame}` (`Spring::{GENTLE, SMOOTH, SNAPPY, BOUNCY, STIFF, new, mass}`, `Animator::{tween, spring, target, snap, cancel, get, value, active}` with `Reactive<f32> for Animator`, and `Owner`-scoped `every_frame`).
- Typed theming, subtree overrides, OS backdrop tracking, and 2D paint transforms: `Token::Custom(&'static str)`, `Theme::for_backdrop(Backdrop)`, `Theme::{control_height, radius, metrics}`, `sigil::style::Backdrop`, `sigil::app::system_backdrop()`, `WindowEvent::BackdropChanged(Backdrop)`, `Widget::theme(impl Reactive<Theme>)`, and `Paint::{translate_x, translate_y, translate, scale, scale_x, scale_y, rotate}`.
- Public `sigil::scene` payload types, so a custom renderer can redraw every command except glyph positions: `Gradient`, `GradientStop`, `GradientKind`, `Stroke`, `StrokeStyle`, `StrokeAlign`, `FillRule`, `PathPaint`, `PathCommand`, `PathStroke`, `CellBorder`, `LayerEffect`, `Filter`, `Mask`, `MaskMode`, `Transform3D`, `DecorationKind`, `PatternKind`, `ShadowKind`, and `TextFlow`. Their fields are private and read through getters: `DrawPath::{outline, fill, fill_rule, stroke}`, `DrawRect::{stroke, border, tone, is_leaf, is_outline}`, the same on `DrawGradient`, `DrawGradient::gradient`, `DrawPattern::pattern`, `DrawImage::placement_at`, `Wash::{paper, pigments, shade}`, `Gradient::{kind, stops, is_repeating}`, `GradientStop::{offset, color}`, `Stroke::{color, width, style, align}`, `PathStroke::{paint, width, cap, join, dash}`, `Shadow::shadow_kind`, and `GlyphRun::{fill, flow}`.
- `Reactive<T>` is a public trait with `read`, `try_static`, `with`, and `into_signal`, implemented for plain values, `Signal<T>`, `Animator`, and closures `Fn() -> T`.
- `tasks::listen(rx, on_message)` returns a `Listener`; `Listener::closed(on_close)` runs once after every sender drops. `rx` is a tokio `mpsc::Receiver` or, on native targets, a `std::sync::mpsc::Receiver`; `sigil::tasks::MessageReceiver` names both.
- `Task::autostart()` starts a task when it is created.
- `number_field` accepts each of the 14 built-in number types (`u8` to `u128`, `usize`, `i8` to `i128`, `isize`, `f32`, and `f64`), named by `sigil::widgets::NumericKind`.
- `sigil::events::ValueStep` and `Widget::step_keys(keys, on_step)` bind keys to unit, page, start, and end steps of a ranged value. `ValueStep::SLIDER_KEYS` is the key table the built-in slider uses. `Widget::accessible(name)` sets the name a screen reader announces on any builder.
- `EmptyStateBuilder` with `description` and `action`, `SkeletonBuilder::lines`, `ProgressBuilder::max`, and `PathBuilder::fill_color`.
- `NodeBuilder::children(iter)` for adding children from any `IntoIterator<Item = impl IntoNode>`.
- `vega(source)` widget constructor in `sigil::widgets` and `sigil::prelude::*` for rendering reactive Vega and Vega-Lite specifications.
- `ColorScale::from_steps`, `ColorScale::on_solid`, `ColorScale::step`, `ColorScale::steps`, and `ColorScale::solid_ink`, plus `Theme::current`, `Theme::with`, `Token::ALL`, `Color::{TRANSPARENT, BLACK, WHITE, alpha}`, and `From<&'static str> for ColorValue`.
- `Host::apply_scale`, `Host::scale`, `Host::apply_theme`, and `Host::node_bounds`, plus automatic `WindowEvent::ScaleFactorChanged` handling in `Host::route`.
- `Signal::try_peek`, `Signal::is_alive`, `EventChannel::new`, `EventChannel::emit`, `Status::{ok, err, as_ref, map}`, and `AsyncRuntime::has_pending`.
- `sigil::prelude::*` exports `Background`, `Easing`, `Rect`, `Selection`, `Transition`, `TreeNode`, `canvas`, `disclosure`, and `vega`.
- Universal `From<*Builder> for NodeBuilder` (including `ImageLoadBuilder` and generic `SliderBuilder<T>`), along with `Paint` and `States` coverage across widget builders.
- `Key` implements `From<&str>`, `From<char>`, `PartialEq<&str>`, and `PartialEq<char>` so `Shortcut::primary('k')`, `Shortcut::ctrl("s")`, and `event.key() == '?'` work directly.
- `Selection` implements `Default` and `From<SelectionMode>`.
- `sigil_kittest::Target` and `Query` accept `String`, `&String`, `Role`, `(Role, &str)`, and `(Role, String)`.

### Changed

- Moved `Cx`, `Live`, `Node`, `NodeId`, `Surface`, `TopLayer`, and `announce` from the crate root (`sigil`) to `sigil::node`, leaving the 9 application entry items at the crate root.
- Re-exported `HostMode` at its canonical home `sigil::app::HostMode`.
- Replaced `IntoPx` with `impl Into<sigil::layout::Length>` on `VirtualListBuilder::{row_height, estimate, viewport_height}` and `TableBuilder::{row_height, viewport_height}`.
- Trimmed 63 specialist items (plus the leaked `prelude::math` submodule) from `sigil::prelude` to 157 items; the removed items remain public in their owning modules (`sigil::{node, layout, geometry, style, events, animation, tasks, scene, app, widgets}`).
- `sigil::widgets::canvas` now passes `(scene, rect, theme)` directly and `CanvasBuilder::themed` is removed; `CanvasBuilder` implements `Framed` (`Paint`, `States`, `Widget`); `SceneCommand::DrawShadow` and `Clip::Rounded` take `[f32; 4]` corner radii.
- `KeyEvent::stop_propagation()` no longer cancels host `Tab` focus movement, `Escape` selection clear, or `Copy` (use `KeyEvent::prevent_default()`); `Widget::shortcut` registers both global shortcut routing and accessibility shortcut metadata.
- Replaced raw geometry tuples with `Point`, `Size`, and `Rect` across `Widget::drop_target` (`DropEvent<T>`), `DragState::position -> Point`, `MenuBuilder::anchor(impl Into<Point>)`, `Host::{layout, frame, scene}(impl Into<Size>, ...)`, and `FrameInputHook -> Option<Size>`.
- `Widget::drive` now accepts `impl Reactive<f32>` so `Animator` and closures plug directly into `.drive(Property::..., value)`.
- `SpringPreset` (feature `kittest`) and the crate-private `Spring` are one public type, `sigil::animation::Spring`.
- Replaced stringly-typed `Theme::token(&str, impl Into<ThemeValue>)` with typed `Theme::token(Token, Color)`, deleted `ThemeValue`, removed the five `Theme` `Length` config-echo getters (`control_width`, `control_height(&self, ControlSize)`, `control_padding`, `control_gap`, `radius(&self, ControlSize)`), unified `sigil::terminal::Backdrop` with `sigil::style::Backdrop`, and upgraded `Widget::theme` to `impl Reactive<Theme>`.
- `Spring::new` and `Spring::mass` no longer panic. NaN, infinite, zero, and negative values are clamped, and a running spring holds its natural frequency to 6-1000 rad/s and its damping ratio to 0.3-1.0, so an undamped or heavily overdamped spring moves as the nearest spring in that range. The presets are unchanged.
- Built-in `button` and `slider` set spacing with the public `Space::raw`, and `axis_drag`, the scrollbar thumb, and the `mermaid` lightbox drag run on `Widget::pointer_drag`. Drag handles no longer expose an accessible Click action, and a release with no active drag passes through.
- A failed copy or cut in a single-line `text_field` logs a `sigil::clipboard` warning, as multiline does.
- `for_each` takes `impl Reactive<Vec<T>>` (a signal, a `memo`, or a closure), `ButtonBuilder::pending` takes `impl Reactive<bool>`, and `Paint::{background, text_color, border_color}` take a reactive value, so a signal or a closure of a `Token` sets a color that follows state.
- `command_menu(open)` takes a `Signal<bool>` and shows the palette while it is true. Choosing an item, Escape at the top level, or a backdrop press sets it to false.
- Constructors take the primary value first and name the rest with setters: `radio_group(selected, options)` with a `Signal<usize>` index into `options`, `progress(value)` with `.max(max)` (default 1.0) and a visible `.label(text)`, `empty_state(title)` with `.description(text)` and `.action(node)`, `skeleton()` with `.lines(n)` (default 3), `tag_input(selected, options)`, and `stepper(current, steps)`.
- `PathBuilder::fill(color)` is `PathBuilder::fill_color(color)`, so it no longer shadows the `Widget::fill` layout setter.
- The setter that sets only the screen-reader name is `accessible` on `NodeBuilder`, `TextBuilder`, `ImageBuilder`, `segmented`, `icon`, `dropdown`, `tabs`, `tree`, and `menu`, and on every builder through `Widget::accessible`. `label` stays the visible caption, which is also the accessible name, on `checkbox`, `toggle`, `gauge`, `progress`, and `slider`.
- Every `accessible` setter takes `impl Into<AccessibleName>`. `toggle`, `segmented`, `dropdown`, `menu`, and `tree` use `Widget::accessible`; `icon` and `tabs` keep their own because the name lands on an inner node.
- `SliderBuilder::label` takes `impl Into<TextContent>`, as the other visible captions do.
- `Token::index` returns `Option<usize>`: the position in `Token::ALL`, or `None` for `Token::Custom`.
- `text(closure)` accepts closures returning any `S: Into<String>` (including `&str`), and `TextContent` implements `From<&String>` and `From<Cow<'_, str>>`.
- `Reactive<T>` supports scalar primitives, `Tone`, `Color`, `ControlSize`, `Layer`, `Token`, `Density`, `DiffView`, `Disclosure`, `&'static [T]`, and `Option<T>`.
- `markdown`, `html`, `toast`, `progress`, and `gauge` accept `impl Reactive<...>` inputs, `scroll` and `axis_drag` accept `impl Into<NodeBuilder>`, `Widget::grid_columns` / `grid_rows` accept `impl Into<Vec<TrackSize>>`, and `Menu::{item, check}` accept `impl Into<Option<Shortcut>>`.
- `segmented`, `dropdown`, and `Combobox::new` accept `impl IntoIterator<Item = impl Into<String>>`, and `tag_input` accepts `impl IntoIterator<Item = impl Into<T>>`.
- `ListRowBuilder::selected`, `ListRowBuilder::disclosure`, `DiffBuilder::view`, and `TableBuilder::density` accept reactive values (`impl Reactive<...>`).

### Fixed

- Tab typed into a multiline `text_field` or the code editor no longer also moves focus to the next control.
- `text_field(..).change` no longer fires for an edit that leaves the text unchanged.
- Ctrl+K and Ctrl+U in a single-line `text_field` delete the selection when there is one, as multiline does.
- An empty IME commit no longer edits a single-line `text_field`.
- Very light, heavily damped, and undamped springs no longer reach non-finite values or move forever.
- `Animator::target` with the value that is already the target no longer restarts a running tween.
- A `drop_hover` signal write no longer panics when a reactive child builds a `drop_target` from it.
- `slider(..).change` and `range_slider(..).change` fire only when the value moves. A key, an assistive edit, a mark press, or a drag that leaves the value where it was no longer calls them.
- A reactive colour set inside `rest`, `hover`, `focus`, `pressed`, or `disabled_paint` follows its signal; before, it was read once.
- The last `background`, `text_color`, or `border_color` setter on a builder wins, whether it is a fixed value or a reactive one.
- `Widget::drive` ignores a NaN or infinite value with a warning and keeps the last finite one; before, it panicked.
- A listener on a `std::sync::mpsc::Receiver` releases the receiver and its thread within 50 ms of its scope ending, and `Listener::closed` runs at once when `listen` has no scope or runtime.
- `number_field` with a reversed range orders the bounds, and a NaN bound is no bound; before, building the field panicked.
- A `text_field(..).password()` that hides its text does not copy or cut it.
- Paste, IME commit, and assistive edits in a single-line `text_field` remove every control character, so a pasted tab no longer enters the field.
- A focused `radio_group` with no checked row shows its focus ring on the first row.
- Leaving a `command_menu` submenu disposes that level's state.
- An open menu panel and a `modal` surface no longer draw a focus ring around the whole panel.
- `sigil-ui` builds with `default-features = false`.
- `children_reactive` and `children_keyed` restore the owning host's cell metrics when reactive updates run outside `Host::enter()`, preventing pixel-sized node rebuilds on terminal/cell hosts.
- `segmented` terminal cell-height calculation to prevent cell-height overflow.
- `app_shell` marks its modal wrapper `.hittable(false)` so closed modals do not swallow clicks, and gives its top and status bands a fixed height on pixel hosts and a minimum height on cell hosts, so `menu_bar()` and `status_bar()` fit on cell hosts and a filling child no longer grows the top band to the window height.
- `Role::PasswordInput` exposes `Action::SetValue` for screen readers and `Harness`.
- `field` resolves `labelled_by` recursively through nested `GenericContainer` wrappers (such as `color_field` and `markdown_edit`) to the inner control.
- Interactive `chip(...).activate(...)` sets its accessible `Role::Button` name from the chip label.
- `tabs` panels set `min_size(0.0, 0.0)` so scrollable tab bodies shrink inside flex containers without overflowing.
- `Host::frame` refreshes `frame.animating` after `paint` so style transitions started during scene emission schedule the next animation tick and do not trigger false frame-0 contrast audit warnings.
- `GridSolver` sizes `Length::Auto` grids to their content height (clamped by `min_size`/`max_size`) and honors Flex `assigned_h` instead of expanding nested grids to the viewport height.
- `divider`, `splitter`, and `sidebar` snap hairline rules and resize handles to cell units on terminal/cell hosts so flex panes and sibling controls do not accumulate sub-cell drift.
- `section_label` and `hint_bar` set `.shrink(0.0)` so 1-row chrome bars do not clip text inside flex columns.

### Migration

| Before | After |
| --- | --- |
| `sigil::{Cx, Live, Node, NodeId, Surface, TopLayer, announce}` | `sigil::node::{Cx, Live, Node, NodeId, Surface, TopLayer, announce}` |
| `sigil::prelude::HostMode` | `sigil::app::HostMode` |
| `sigil::widgets::Layout` (feature `kittest`) | `sigil::widgets::Widget` (`Widget::layout`) |
| `sigil::widgets::Orientation` | `sigil::layout::Orientation` (also in `sigil::prelude::*`) |
| `HitPosition { byte_index, affinity }` | `HitPosition::new(byte_index, affinity)` / `HitPosition::downstream(byte_index)`, `.byte_index()`, `.affinity()` |
| `CaretGeometry { x, top, height }` | `CaretGeometry::new(x, top, height)`, `.x()`, `.top()`, `.height()` |
| `canvas(\|scene, rect\| ...)` / `canvas(...).themed(...)` | `canvas(\|scene, rect, theme\| ...)` |
| `scene.commands` (field) | `scene.commands()` |
| `Clip::Rounded(rect, CornerRadii)` / `SceneBuilder::draw_shadow(rect, CornerRadii, shadow)` | `[f32; 4]` corner radii |
| specialist items removed from `sigil::prelude::*` | import from their canonical modules (`sigil::{node, layout, geometry, style, events, animation, tasks, scene, app, widgets}`) |
| `event.stop_propagation()` to cancel `Tab` focus movement / `Escape` clear / `Copy` | `event.prevent_default()` |
| `NodeBuilder::keyboard_shortcut(s)` + `.key(s, f)` | `Widget::shortcut(s, f)` (or `Widget::accessible_shortcut(s)` for passive screen-reader hint only) |
| `Widget::position_reactive(sig)` | `Widget::offset(sig)` with `Point` (`impl Reactive<Point>`) |
| `Widget::{bounds_reactive, size_reactive}(sig)` | `Widget::{track_bounds, track_size}(sig)` with `Signal<Rect>` / `Signal<Size>` (plus `Widget::track_position` with `Signal<Point>`) |
| `Widget::drop_target(state, accepts, \|value\| ...)` | `Widget::drop_target(state, accepts, \|event: DropEvent<T>\| ...)` (use `event.into_value()`, `event.local_position()`, `event.bounds()`) and `Widget::drop_hover` |
| `DragState::position() -> (f32, f32)` | `DragState::position() -> Point` |
| `MenuBuilder::anchor(x, y)` | `MenuBuilder::anchor(impl Into<Point>)` |
| `Host::{layout, frame, scene}((w, h), ...)` | `Host::{layout, frame, scene}(impl Into<Size>, ...)` (`(w, h)` still works via `From<(f32, f32)>`) |
| `FrameInputHook` returning `Option<(f32, f32)>` | `FrameInputHook` returning `Option<Size>` |
| `SpringPreset::{Gentle, Smooth, Snappy, Bouncy, Stiff}` (feature `kittest`) | `Spring::{GENTLE, SMOOTH, SNAPPY, BOUNCY, STIFF}` in `sigil::animation` |
| `Animator::spring(signal, preset)` / `Animator::tween(signal, ms, easing)` | `Animator::spring(initial, spring)` / `Animator::tween(initial, transition)` (read with `.get()` or `.value().peek()`) |
| `Animator::settled()` / `Animator::finish()` | `!anim.active().peek()` / `anim.snap(value)` |
| `Theme::token("name", color)` / `ThemeValue` | `Theme::token(Token, Color)` with `Token::Custom("name")` (`ThemeValue` removed) |
| `Theme::{control_width, control_height, control_padding, control_gap, radius}(&self, ControlSize)` | `Theme::{control_height(ControlSize, f32), radius(Radius, f32)}` setters and `theme.metrics()` (`Metrics::{control_width, control_height, control_padding, control_gap, radius}`) |
| `sigil::terminal::Backdrop` (separate enum) | `sigil::style::Backdrop` (re-exported at `sigil::terminal::Backdrop`) with `Theme::for_backdrop(Backdrop)` |
| Manual dark/light OS polling | `sigil::app::system_backdrop()` and `WindowEvent::BackdropChanged(Backdrop)` |
| `Widget::theme(Theme)` | `Widget::theme(impl Reactive<Theme>)` (accepts `Theme`, `Signal<Theme>`, or closure `move \|\| Theme`) |
| `Paint` 3D-only transforms | Added 2D `Paint::{translate_x, translate_y, translate, scale, scale_x, scale_y, rotate}` |
| `PathCmd` | `sigil::scene::PathCommand` |
| field reads of `Gradient` (`kind`, `stops`, `repeat`), `GradientStop`, `Stroke`, and `PathStroke` (feature `kittest`) | getters of the same names: `gradient.stops()`, `stop.offset()`, `stroke.color()`, `path_stroke.dash()`; `repeat` is `is_repeating()` |
| `DrawRect::stroke_style()`, `DrawRect::cell_border()` (feature `kittest`) | `DrawRect::stroke()`, `DrawRect::border()`, public; the `kittest` setter `stroke(stroke)` on `DrawRect`, `DrawGradient`, and `DrawPattern` is `stroked(stroke)` |
| `Token::index() -> usize` | `Token::index() -> Option<usize>`; `None` for `Token::Custom` |
| `.label(name)` for the screen-reader name on `NodeBuilder`, `TextBuilder`, `ImageBuilder`, `segmented`, `icon`, `dropdown`, `tabs`, `tree`, `menu`, and `radio_group` | `.accessible(name)`; `label` on `checkbox`, `toggle`, `gauge`, `progress`, and `slider` is unchanged |
| `command_menu()` inside a `modal(open)` | `command_menu(open)`; it shows while `open` is true |
| `radio_group(selected: Signal<String>).option(key, label)` | `radio_group(selected: Signal<usize>, [label, ..])`; `selected` is the index of the chosen option |
| `progress(value, max)` | `progress(value).max(max)`; `.label(text)` shows the text before the bar |
| `empty_state(title, body)` | `empty_state(title).description(body)` |
| `skeleton(lines)` | `skeleton().lines(lines)` |
| `tag_input(options, selected)` | `tag_input(selected, options)` |
| `stepper(steps, current)` | `stepper(current, steps)` |
| `PathBuilder::fill(color)` | `PathBuilder::fill_color(color)` |
| `Host::misalignments`, `Host::emit_since`, `Cx::take_delta`, `LayoutStyle::data` | `sigil::testing::misalignments`, `emit_since`, `take_delta`, `style_data` (feature `kittest`) |
| `LayoutStyle::with_layout`, `events::EventRouter`, `style::StyleBinding`, `style::ReactiveStateSlot`, `style::ReactiveStyleAtoms` | public only under feature `kittest` |
| `GlyphRun::positioned_glyphs` | `GlyphRun::glyph_count` for the count; the glyphs are crate-private |
| `Node::tag`, `TextStyle::layered_over_binding`, `FrameInfo::delta` | crate-private |
| `sigil::kernel::*` | `sigil::node` (`text`, `TextBuilder`, `ImageBuilder`, `ObjectFit`, `Cx`, `NodeId`, `Node`, `AccessibleName`, `TopLayer`, `Surface`, `Live`, `announce`) and `sigil::app::HostMode`; `NodeType`, `NodeExtensions`, `ImageCrop`, `ImageRaster`, and `FocusScopeKind` are public only under `kittest` |
| `sigil::animation_runtime::{Animator, AnimatorStatus, Playback, clock, reduced_motion}` | `sigil::animation::{Animator, clock, reduced_motion}`; `AnimatorStatus` and `Playback` are removed |
| `app::DefaultTextMeasure`, `profile::{note_input, FRAME_BUDGET_NS}`, `widgets::MIN_COL_WIDTH`, `DrawRect::border_radius` | public only under `kittest`, `#[doc(hidden)]`; `TextMeasure`, `CaretGeometry`, `HitPosition`, and `TextAffinity` are public in `sigil::text`, and `DrawRect::border()` is public |
| `sigil::text::{WebFont, WebFontScript, add_fallback_font, load_font_url, set_web_fallbacks, web_fallbacks}` | `sigil::app::*` |
| `sigil::a11y::snapshot(&host.accessibility_tree())` | `host.accessibility_text()` |
| `sigil::a11y`, `sigil::kernel`, other items in them | public only under `kittest`, `#[doc(hidden)]`; the node items are in `sigil::node`, and `sigil::layout` and `sigil::text` are public |
| `layout::Rect<T>` (edges), `layout::Size<T>` (per axis) | `Edges<T>`, `Axes<T>`, under `kittest`. `geometry::Rect` stays the box |
| `style::DisplayKind`, `FlexWrapKind`, `OverflowKind`, `AlignKind`, `JustifyKind` | `Display`, `FlexWrap`, `Overflow`, `AlignItems`, `JustifyContent`. `JustifyKind::Between`, `Around`, `Evenly` are `JustifyContent::SpaceBetween`, `SpaceAround`, `SpaceEvenly`. `AlignKind::Between` and `Around` are gone; the class grammar never produced them |
| `AlignItems::FlexStart`, `AlignItems::FlexEnd` | `AlignItems::Start`, `AlignItems::End` |
| `ColorOrToken`, `ColorKey`, `ColorValue::ThemeRef(ColorKey)` | `ColorValue::{Color, Token, Custom}`. `ColorOrToken::Token(ColorKey::Custom(name))` is `ColorValue::Custom(name)` |
| `StyleAtom::BackgroundThemeRef`, `TextThemeRef`, `BorderThemeRef` | `StyleAtom::BackgroundColor`, `TextColor`, `BorderColor` |
| `Theme::resolve_color(ColorKey)`, `TokenLayer::color(ColorKey)` | take `ColorValue` |
| `.animate(NumericAtomKind::Height, value)` | `.drive(Property::Height, value)`. `ColorAtomKind` is private |
| `NumericAtomKind::X` in `.drive(NumericAtomKind::X, value)` | `sigil::animation::Property::X`, the enum that `Keyframes` uses: `.drive(Property::Width, value)`. `Rotation`, `RotationX`, `RotationY`, `RotationZ` are `Rotate`, `RotateX`, `RotateY`, `RotateZ`; `StopFrom`, `StopVia`, `StopTo` are `GradientFromPosition`, `GradientViaPosition`, `GradientToPosition`; `TranslateXFraction`, `TranslateYFraction` are `TranslateXPercent`, `TranslateYPercent` and take percent. `NumericAtomKind` is private. `Keyframes` ignores the layout properties (`Width`, `Height`, `Gap`, `PaddingX`, `PaddingY`, `FontSize`, `LineHeight`) with a `sigil::animation` warning |
| `Rgba8: FromStr` accepted `#` plus 3 or 6 digits | `Color: FromStr` also accepts 8 digits and a `0x` or missing prefix. Theme files use it, so they also accept 3 digits |
| `SceneRectStyle`, `SceneGradientStyle`, `ScenePatternStyle`, `SceneSpriteStyle`, `SceneDecorationStyle` | setters on `DrawRect`, `DrawGradient`, `DrawPattern`, `DrawSprite`, `DrawDecoration` |
| `scene.draw_rect(rect, SceneRectStyle::solid(color))` | `scene.draw_rect(DrawRect::new(rect, color))`. `draw_gradient`, `draw_pattern`, `draw_decoration`, and `draw_sprite` take the draw command too |
| `DrawGlyphRun`, `draw.run.text` | `SceneCommand::GlyphRun(GlyphRun)`, `draw.text` |
| `Axis::X`, `Axis::Y` | `Orientation::Horizontal`, `Orientation::Vertical` |
| `ToastLevel`, `ToastEntry::level`, `ToastLevel::Warning` | `Tone`, `ToastQueue::push(tone, message)`, `Tone::Warning` |
| `SortDir`, `kernel::SortDirection` re-exported from `accesskit` | `SortDirection`, owned by Sigil, without an `Other` variant |
| `HlSpan { token: &'static str }`, `hl_spans`, `highlight` byte ranges | `Highlight { token: Token }` from `highlight`, in the grapheme columns `code_edit` takes |
| public `install_animators` | removed; no production caller |
| `Role`, `Action`, `Live` re-exported from `accesskit` | Sigil enums. `Role::from_accesskit` reads a role from `Host::accessibility_tree`. `Action` has the variants the widgets use |
| `Key`, `PhysicalKey`, `ModifiersState` re-exported from `keyboard-types` | Sigil types with the same variant and method names |
| `SmolStr` in `LogicalKey`, `KeyEvent::text`, `ImeEvent`, and `Event::text` | `String` |
| `Cell::text`, `Cell::link` fields; `Cell { text, ..Cell::blank() }` | `Cell::text()`, `Cell::link()`, `Cell::set_text`. Build a cell from `Cell::blank()` |
| `app::menu` re-exports of `muda`, `accelerator`, `set_menu_bar(menu)`, `events() -> EventChannel<MenuId>` | `Menu` with `item`, `disabled`, `check`, `standard`, `separator`, `submenu`, and `install`; `Menu::events() -> EventChannel<String>` |
| `app::tray` re-exports of `tray-icon`, `set_tray(TrayIconBuilder)`, `update(FnOnce(&TrayIcon))`, `events()` | `Tray`, `TrayEvent`, `Tray::install(self) -> AppResult<()>`, `Tray::update(&self) -> AppResult<bool>`, `Tray::clear_icon`, `Tray::clear_menu`, `Tray::events()` |
| `app::notify::send(&notification)` | `notification.send()` |
| `widgets::HintEntry` | removed; `hint_overlay` takes `&[(&str, &str)]` |
| `Easing::Bezier`, `animation::clock::clock`, `profile::frame_budget_ns` | removed. `animation::clock()` stays |
| `style::{SOLID_VARIANT, SOFT_VARIANT, GHOST_VARIANT, SELECTION_FILL, SPACING_STEP}`, `Theme::with_theme_tokens`, `terminal::CONTINUES`, `app::display_scale`, `profile::{attribute, gpu_pass, dump_if_enabled, Hotspot, BUDGET_ENV, DEFAULT_BUDGET_MS, MAX_SUBJECTS, OTHER_SUBJECT, INTERACT_BUDGET_NS}` | crate-private |
| `app::AccessibilityTree` | `accesskit::TreeUpdate` |
| `a11y::AccessKitBridge`, `node`, `tree`, `BoundsLookup` | `#[doc(hidden)]` under `kittest` |
| `a11y::RecordingBridge` | removed; no caller |
| `From<wgpu::RequestDeviceError>`, `From<wgpu::CreateSurfaceError>`, `From<winit::error::EventLoopError>` for `Error` | removed |
| `OverflowKind::Auto` stored as `Overflow::Scroll` | `Overflow::Auto` |
| `render_widget_to_png*`, `capture_widget_scaled_focus_descendant`, `Harness::render_stub`, `render_with`, `render_scaled`, `render_scaled_focus`, `render_focus`, `capture_scaled_focus` | `Harness::render(build)` with `.mode`, `.size`, `.scale`, `.hit`, `.measure`, `.stub`, `.focus_first`, then `.image()` or `.capture(&mut settle)` |
| `sigil_kittest::Captured` | the tuple `(RgbaImage, Option<Rect>)` from `Render::capture` |
| `capture_filmstrip` | `Harness::render(build).size(..).hit(..).filmstrip(frames, dt)` |
| `golden_path!`, `golden_path_in`, `assert_rendered_matches_golden`, `assert_png_matches_golden`, `assert_goldens_differ!`, `assert_goldens_differ_in` | `Golden::new(stem)`, `.delta(pct)`, `.path()`, `.assert(rendered)`, `.assert_image(&image)`, `.assert_differs(&other)` |
| `assert_text_matches_golden`, `regen_mode`, `regen_enabled`, `Regen`, `SNAPSHOT_MAX_DELTA_PCT`, `FILMSTRIP_GAP_PX` | `Harness::snapshot` on a cell harness; the rest are crate-private |
| `app::snapshot(view, size)` | `Harness::new(view).pixels(w, h).image()` |
| `terminal::present` | `Harness::new(tree).size(cols, rows).text()` |
| `terminal::step(host, events, cols, rows)` | `host.grid(events, cols, rows)` |
| `terminal::render(out, prev, next, depth)` | `next.write_ansi(out, prev, depth)` on `terminal::Grid` |
| `terminal::raster`, `raster_into`, `raster_with`, `terminal::Paint`, `terminal::Marks` | `Host::paint_cells(&mut grid, &scene)`; the rest are crate-private |
| `layout::TextGeometry`, `app::EngineGeometry` | `layout::TextMeasure` with `hit`, `caret`, and `selection_rects`; `app::DefaultTextMeasure` |
| `sigil_kittest::FixedMeasure { width, height }`, `CharMeasure { char_width, line_height }` | `FakeMeasure::fixed(width, height)`, `FakeMeasure::chars(char_width, line_height)` |
| `text::prefer_system_fonts`, `app::use_system_fonts` overriding custom fonts | `app::use_system_fonts`, which keeps fonts set with `set_default_fonts` |
| `AnimationConfig` | `Animator::spring`, `Animator::tween`, `presence_spring`, `presence_tween` |
| `set_mouse_capture` | `set_mouse_tracking` |
| `install_animators_themed` | removed; no production caller |
| `text::TextConfig` | `style::TextStyle`. The old engine default is `TextStyle { font_size: 16.0, ..TextStyle::default() }` |
| `bidi` | crate-private. Text layout still uses it |
| `virtual_list_bind` | `virtual_list` |
| `ShellEvent` | `Event` |
| `FlexDir`, public `AlignSelf` | `FlexDirection`, `AlignItems` |
| `parse_theme`, `parse_hex` | `Theme: FromStr`, `Color: FromStr`. 3 digits and surrounding whitespace are newly accepted, and `+12345` is newly rejected |
| `pub use wgpu` | removed |
| functions that returned `std::io::Result` | return `sigil::Error` |
| `Dimension::px` | removed; pass `impl Into<Length>`. `Length` does not implement `Eq` or `Hash` |
| `StyleAtom::Width`, `Height`, `MinWidth`, `MaxWidth`, `MinHeight`, `MaxHeight` hold `layout::Dimension` | each holds `(u8, i16)`, the discrete atom encoding. A second field was added to each variant |
| `Dimension::Auto` or `Dimension::Fill` used as padding, gap, or border | stored as `Auto` or `Fill`, not converted to `0px` |
| `sigil::layout::LengthPercentage`, `sigil::layout::LengthPercentageAuto` | `layout::Dimension` on `StyleData` padding, margin, gap, inset, and border. `inset_bottom(Dimension::Percent(1.0))` replaces `LengthPercentageAuto::Percent(1.0)` |
| `sigil::layout::TextLength`; `rows`, `cols`, and `em` return `TextLength`; `Dimension::Text` | `rows`, `cols`, and `em` return `Dimension::Rows`, `Dimension::Cols`, and `Dimension::Em` |
| `Dimension` without `Rows`, `Cols`, and `Em` | `Dimension` adds `Rows`, `Cols`, and `Em` |
| `sigil::style::Dimension`, `sigil::style::Px` | a pixel atom and a `RenderStyle` border field are `i16`; `BORDER_WIDTH` and `ICON_SIZE` are `i16`; `Metrics::radius` returns `f32` |
| `sigil::geometry::Dim`; public `Size.width` and `Size.height` | `Size::fixed`, `Size::min_content`, `Size::max_content`, `with_fixed_width`, `with_min_content_width`, `with_max_content_width`, `fixed_width`, `fixed_height` |
| `Host::frame(&mut router, viewport, dt, hooks)` | `Host::frame(viewport, dt, hooks)`; `Host` owns its event router |
| `Host::route(&mut router, event)`, `Host::perform(&mut router, id, &request)`, `Host::focus_intent(&mut router, intent)`, `Host::repair_focus(&mut router)` | the same methods without the router argument |
| `Host::accessibility_tree(&router)`, `router.focus()` | `Host::accessibility_tree()`, `Host::focus()` |
| `KeyEvent::new(KeyEventInit { .. })` | `Event::key(key, phase)` with `physical`, `modifiers`, `repeat`, `text` setters |
| `PointerEvent::new(PointerEventInit { .. })`, `PointerEvent::with_contact` | `Event::pointer(phase, position)` with `delta`, `kind`, `button`, `buttons`, `modifiers` setters. `with_contact` is crate-private |
| `ScrollEvent::new(ScrollEventInit { .. })` | `Event::scroll(position, delta, phase)` with the `modifiers` setter |
| `GestureEvent::new(GestureEventInit { .. })` | `Event::gesture(kind, phase, position)` with the `modifiers` setter |
| `PasteEvent`; `on_paste(Fn(&mut PasteEvent))` | `paste(Fn(&str) -> bool)`; return true where the handler called `stop_propagation`. `Event::paste(text)` builds the event |
| `terminal::key_down(key)`, `terminal::paste(text)` | `Event::key_down(key)`, `Event::paste(text)` |
| `terminal::pointer(phase, col, row)`, `terminal::click(col, row)`, `terminal::wheel(col, row, lines)` | `Event::pointer_down`, `pointer_up`, `pointer_drag`, `click`, `wheel` at the layout point of the cell (`Host::CELL` points per cell). `Event::click` also sends the move before the press |
| `sigil_kittest::input::{pointer_move, pointer_drag, pointer_down, pointer_up, click}(x, y)` | `Event::pointer_move`, `pointer_drag`, `pointer_down`, `pointer_up`, `click` with `Point::new(x, y)` |
| `input::key_event(key, physical, phase)`, `key_down(key, physical)`, `chord(key, physical, modifiers)` | `Event::key(key, phase).physical(physical)`, `Event::key_down(key).physical(physical)`, adding `.modifiers(modifiers)` for a chord |
| `input::press`, `type_char(text, physical)`, `type_text`, `tab`, `scroll_lines(x, y, dy)`, `window_blur`, `ime_event(event)` | `Event::press`, `Event::typed(text).physical(physical)`, `Event::type_text`, `Event::tab`, `Event::wheel(Point::new(x, y), dy)`, `Event::blur`, `Event::Ime(event)` |
| `app::TextTuning`, `set_text_tuning`, `text_tuning` | removed from the app API; no caller tuned glyphs. Tests keep `set_text_tuning` under feature `kittest` |
| `app::DefaultFonts`, `set_default_fonts` | `AppConfig::font(FontFamily, bytes, face)`, `font_sans_weight`, `font_fallback` |
| `animation_runtime::MotionPrefs::reduced()`, `MotionPrefs::set` | `animation::reduced_motion()`, `AppConfig::reduced_motion` |
| `sigil::animation_runtime::{MotionPrefs, active_count, presets, running_clocks, tick}` (feature `kittest`) | `sigil::animation::*`, under `kittest`, `#[doc(hidden)]`. The `sigil::animation_runtime` module is removed |
| `style::StyledTextEngine` (feature `kittest`) | removed; call `OwnedTextEngine::layout_paragraph_styled`, no trait import |
| `layout::TextEngineMeasure<OwnedTextEngine>` (feature `kittest`) | `layout::TextEngineMeasure` |
| `AsyncRuntime::drain` (feature `kittest`) | `AsyncRuntime::pump` |
| `app::HostCaps`, `Host::set_caps(HostCaps { hyperlinks })`, `Caps::host()` | `Host::hyperlinks(hyperlinks)` |
| `app::WindowOpts` | `AppConfig::overlay`, `frameless`, `anchor`, `user_zoom`, `min_size`, `resizable` |
| `reactive::effect_named` | `reactive::effect`. Named effects are crate-private |
| `tasks::on_message(rx, f)`, `on_message_or_close(rx, f, on_close)` | `tasks::listen(rx, f)`, and `.closed(on_close)` on the `Listener` it returns |
| `task_with`, `Task::run_with`, `Task<T, E, A>` | `task` and `Task<T, E>`; capture the argument in a signal the closure reads |
| `reactive::cancel_interval(id)` | `IntervalId::cancel` |
| `kernel::announce(text)`, `announce_assertive(text)` | `kernel::announce(text, Live::Polite)`, `announce(text, Live::Assertive)` |
| `scene::emit_scene(cx, root)`, `emit_scene_clipped(cx, root, clip)` | `scene::emit_scene(cx, root, clip)`; pass `None` for no clip |
| `list_row(selected, title, detail)`, `list_row_with(selected, title, slots)`, `ListRowSlots::row` | `list_row(title).selected(selected)`; `detail` is `.status(text)` |
| `image_rgba(w, h, rgba)`, `image_bytes(encoded)` | `ImageSource::from_rgba8(w, h, rgba).map(image)`, `ImageSource::decode(encoded).map(image)` |
| `app::open_window(config, view)` | `WindowId::open(config, view)` |
| `Host::new()`, `Host::cells()`, `Host::default()` | `Host::new(HostMode::Pixel)`, `Host::new(HostMode::Cell)` |
| `tasks::TaskWorkers`, `AsyncRuntime::with_workers`, `AppConfig::task_workers(TaskWorkers)` | `AppConfig::task_workers(NonZeroUsize)` |
| Feature `render` | Feature `scene` |
| `AppConfig::title()`, `window_size()`, `exits_on_esc()`, `on_panic()` | removed. The app sets these values and keeps them |
| `Host::apply_focus_requests`, `Host::sync_hover` | `Host::frame` runs them |
| `FrameInputHook`, `FrameLayoutHook`: `FnOnce(&mut Host, &mut EventRouter)`; `FramePaintHook`: `FnOnce(&mut Host, &mut EventRouter, &FrameInfo)` | `FnOnce(&mut Host)`; `FnOnce(&mut Host, &Frame)` |
| `terminal::Screen::frame(host, router, events, size)`, `terminal::step(host, router, events, cols, rows)`, `terminal::AfterFrame` with a router argument | the same without the router |
| `sigil_kittest::Harness::host() -> (&mut Host, &mut EventRouter)` | `Harness::host() -> &mut Host` |
| `reactive::Runtime::scope(name, f)` | `reactive::Owner::scope(name, f)` |
| `reactive::OwnerId`, `tasks::OwnerHandle`, `Owner::id()`, `OwnerHandle::from_id(id)` | `reactive::Owner`, a `Copy` id that is `Send` |
| `OwnerHandle::run(f)` | `if owner.is_alive() { owner.with(f) }` |
| `Owner::new(name, Option<OwnerId>)`, `Owner::current() -> Option<OwnerId>` | `Owner::new(name, Option<Owner>)`, `Owner::current() -> Option<Owner>` |
| `style::NodeStyleExt` (`theme`, `tone`, `focus_ring`, `presence`, `animate`, `visible_when`, `paint_when`) | `Widget`, on every builder |
| `layout::LayoutBuilderExt` (`layout`, `asymmetric`) | `Widget` |
| `layout::LayoutReactiveExt` (`position_reactive`, `size_reactive`, `bounds_reactive`) | `Widget` |
| `events::NodeEventsExt` (event handler methods) | `Widget` |
| `widgets::DragDropExt<T>` (`drag_source`, `drop_target`) | `Widget`; the methods are generic over `T` |
| `widgets::layout_ext::WidgetLayoutExt` (`layout`, `width`, `height`, `grow`, `map_node`), `T: WidgetLayoutExt` bounds | `Widget`, `T: Widget` |
| `style::StateVariants` (`rest`, `hover`, `focus`, `pressed`, `disabled_style`) | `style::States` (`rest`, `hover`, `focus`, `pressed`, `disabled_paint`), implemented only for a builder that has state slots |
| `NodeBuilder::tone(Tone)`, `TextBuilder::tone(Tone)` (from `NodeStyleExt`) | `Widget::tone_scope(Tone)`; `tone` stays the widget tone on button, chart, chip, checkbox, icon, progress, radio group, sparkline, and stat |
| `TooltipBuilder::shortcut(Option<impl Into<String>>)` | `TooltipBuilder::hint(Option<impl Into<String>>)` |
| `CodeEditBuilder::key(f)` | `CodeEditBuilder::key_handler(f)` |
| `ChipBuilder::layout(LayoutStyle) -> NodeBuilder` | the `Widget` layout setters, which return the `ChipBuilder` |
| `CanvasBuilder::layout` | the `Widget` layout setters |
| `PaneBuilder::grow`, `TextFieldBuilder::autofocus`, `TextFieldBuilder::focus_when`, `TextEditBuilder::autofocus`, `TextEditBuilder::focus_when` | the `Widget` methods of the same name, with the same effect |
| `VerticalDivider`, `divider_vertical() -> VerticalDivider` | `divider(Orientation::Vertical) -> NodeBuilder` |
| `PathNode::layout(LayoutStyle)`, which replaced the `fit` box | the `Widget` layout setters, which merge into it |
| `widgets::layout_ext::LayoutOverrides` | The `Widget` layout setters, such as `width`, `height`, and `grow` |
| A second `.layout(..)` on a widget builder replaced the first | Each layout setter merges, in call order, as on `NodeBuilder` |
| `const fn color_field` | `fn color_field` |
| `const fn divider_vertical` | `fn divider(Orientation)` |
| `widgets::layout_ext` | private; its methods are `Widget` |
| `TextFieldBuilder::autofocus`, `TextFieldBuilder::focus_when`, `TextEditBuilder::autofocus`, and `TextEditBuilder::focus_when` as `const fn` | the same methods, no longer `const` |
| `Placement` without `mark_col`, `mark_row`, `span_cols`, and `span_rows` | those fields are part of `Placement` |
| `ColorFieldBuilder` and `MarkdownEditBuilder` are `UnwindSafe` | they are not; their callback fields are trait objects |
| `TextBuilder::label`, `description`, and `keyboard_shortcut` as `const fn` | the same methods, no longer `const`; `label` is `accessible` |
| `text(...).map_node` and `image(...).map_node` run at the call | they run when the node is built, after `layout`, `width`, `height`, and `grow` |
| a width or height hint ignores a size set through `layout` | it follows a size set through `layout` as well as through `width` and `height` |
| bound traits `IntoPx`, `IntoIconSource`, `IntoScrollExtent` | removed; the parameters take `impl Into<Length>`, `impl Into<Glyph>`, and `impl Reactive<f32>`, so the same values work |
| `animation::Tween` | crate-private; `Animator` and `Spring` are public in `sigil::animation` |
| `animation::CubicBezier` | removed with `Easing::Bezier` |
| `PathNode` | `PathBuilder` |
| `Harness::gpu(width, height, scale)` | `gpu(width, height).scale(scale)` |
| `Harness::press_with(key, modifiers)` | `press(Shortcut::new(key, modifiers))`; `press` takes a `Key`, a `NamedKey` or a `Shortcut`, so a `String` key is `Key::character(text)` |
| `Harness::present(tree, cols, rows, events)` | `Harness::new(tree).size(cols, rows).send(events.iter().cloned()).text()`; it returns a `String`, runs frames until the UI settles, and checks the design rules when the harness is dropped |
| `Harness::snapshot_with(name, pct)` | `Golden::new(name).delta(pct).assert(ui.image())` |
| `ColorAtom` | `ColorValue` |
| `TextFieldBuilder::blur(f)` | `focus_out(f)`, as on every widget; `blur(px)` is the filter |
| `ModalBuilder::show(signal)` | `modal(signal)` |
| `command_menu().item(label, action)`, `.detail(text)`, `.disabled(label)`, and the same on `SubmenuBuilder` | `command_menu(open).item(MenuItem::new(label).hint(text).disabled(true).activate(action))` |
| `app::FrameInfo`, `app::FrameOutput` | `app::Frame`: `Host::frame` returns it and the paint hook receives it; `info.layout_ok` is `frame.layout().is_ok()` |
| `Host::ansi(out, prev, next, depth)` | `next.write_ansi(out, prev, depth)` |
| `.asymmetric(reason)` | `.allow_flaw(Flaw::Column, reason)` or `.allow_flaw(Flaw::Inset, reason)` |
| `TextStyle { font_size: 16.0, ..TextStyle::default() }` | `TextStyle::default().font_size(16.0)`; one setter per field |
| struct literals of `scene::DrawRect`, `scene::Shadow`, `events::Shortcut`, `events::PointerTilt` | these are `#[non_exhaustive]`; use their constructors |
| `Surface { key }` | `Surface::new(key)` |
| `FrameHooks { input: Some(Box::new(f)), .. }` | `FrameHooks::default().input(f)`, `.after_layout(f)`, `.paint(f)` |
| `terminal::Hooks { region_rows: Some(Box::new(f)), .. }` | `Hooks::default().region_rows(f)`, `.external_events(f)`, `.after_frame(f)` |
| `terminal::Access { focus_cursor, status }` | `Access::default().focus_cursor(..).status(..)` |
| struct literals of `TextFace`, `TextRun`, `Locale`, `terminal::Cell`, `terminal::Lines`, `scene::GlyphRun`, `ListState` | these are `#[non_exhaustive]`; read the fields, build with `TextRun::new`, `Locale::new`, `GlyphRun::new`, or `default()` and field writes for `TextFace`, `Cell` and `Lines` |
| `ProgressBuilder.indeterminate` field | `.indeterminate()` |
| `Theme` public fields (`theme.accent`, `theme.gray.step(n)`) | `theme.color(Token::Accent)`, `theme.scale(Tone::Neutral).step(n)`; write with `.tone(Tone, ColorScale)` and `.token(Token, Color)`; `theme.direction`, `locale` and `strings` are written with the setters of those names and have no reader; `Metrics::current()` reads the metrics |
| `style::ScaleRole`, `Tone::scale_role` | `Tone` (`Gray` is `Tone::Neutral`, `Accent` is `Tone::Primary`) |
| `SelectionState` with `click`, `ctrl_click`, `shift_click`, `selected_indices` | `Selection` with `select`, `toggle`, `extend`, `indices`, `clear` |
| `TableState::select_only`, `toggle_selected`, `select_range_to`, `clear_selection` | `TableState::select`, `toggle`, `extend`, `clear` |
| diff `Selection` with public `hunks` and `lines` | `DiffSelection` with `hunks()` and `lines()` |
| `States::disabled_style` | `States::disabled_paint` |
| `widgets::close_top_escape`, `widgets::close_popup` | removed; Escape and light dismiss close the most recently opened popup |
| `MouseButtons::EMPTY` | `MouseButtons::empty()` |
| `list_row(selected, title, ListRowSlots { .. })` | `list_row(title).selected(selected)` with one setter per slot; `ListRowSlots` and `ListColumn` removed, `.column(text, width, ColumnAlign)` adds a column |
| `profile::BackendStatus { kind, fallback }` and its fields | `BackendStatus::new(backend, fallback)`, `backend()`, `fallback()` |
| `terminal::title`, `mouse_tracking`, `print_above`, `notify`, `progress`, `bell`, `activity`, `calibrate`, `suspend`, `caps`, `backdrop`, `pixels` free functions | the same names as methods of `terminal::Terminal::current()` |
| `terminal::focused()`, `terminal::set_pixels(on)` | `Terminal::is_focused()`, `Terminal::draw_pixels(on)` |
| field reads of `Patch`, `Hunk` and `Line` (`patch.hunks`, `hunk.path`, `line.old`) | getters: `patch.hunks()`, `hunk.path()`, `hunk.lines()`, `line.kind()`, `line.old_line()`, `line.new_line()` |
| struct literals and field reads of `SortKey`, `Highlight`, `Diagnostic`; field reads of `TreeNode` | `SortKey::new(column, direction)`, `Highlight::new(start, end, token)`, `Diagnostic::new(line, start, end, severity)` and a getter per field (`SortKey::direction()` for `dir`); `TreeNode` is built with `new` and its setters |
| public fields of `Date`, `Time`, `DateTime`, `MenuItem` and `WebFont` | `Date::year()`, `month()`, `day()`; `Time::hour()`, `minute()`, `second()`; `DateTime::date()`, `time()`; `WebFont::url()`, with no reader for its script and face; `MenuItem` and `WebFont` are built with `new` and their setters, and `MenuItem::submenu_indicator` is `submenu(bool)` |
| `app::set_web_fallbacks(fonts)` | `AppConfig::web_fonts(fonts)` |
| `AppConfig::font_sans`, `font_mono`, `font_icon`, `font_display` | `AppConfig::font(FontFamily::Sans, bytes, face)` and the other families |
| `clipboard::clipboard()` | `clipboard::copy`, `paste`, `copy_image`; the handle is crate-private |
| `profile::tick_frame`, `profile::count_add(counter, n)` | `profile::end_frame`, `profile::count(counter, n)` |
| `ButtonBuilder::new` and the `new` of 15 other widget builders | the widget function: `button(label)`, `checkbox(checked)`, `slider(value, range)` |
| `animation::{Keyframe, Keyframes, AnimationDirection, AnimationFill, Playback, AnimatorStatus}`, `terminal::CellGrid`, `style::ThemeBuilder`, `widgets::FocusCaret`, `TextFieldBuilder::focus_caret`, `Widget::presence_spring` | removed; no caller used them. `animation::Keyframes` and `Widget::ime_caret` are public |
| `Paint::{background_gradient, transform, transition, duration}`, `LayoutStyle::{align_content, box_sizing, flex_direction}`, `SceneBuilder::{draw_pattern, draw_sprite, push_layer, draw_decoration, composite}`, `DrawRect::border_radius`, `app::{set_ambiguous, set_cell_width}`, `style::render_style_for` | public only under `kittest`; `SceneBuilder::{draw_gradient, draw_image, draw_shadow}` are public |
| `style::scale`, `style::{StyleAtom, StateStyle, StyleState, StyleVariant, StyleEngine, RenderStyle, Transform2D, TransitionId, MillisU16, BgGradient}` and the other style value types | public only under `kittest` or crate-private; `Token::scale` returns a `Tone` |
| `scene::{DrawSprite, LinearGradient, CornerRadii}` | public only under `kittest`; `SceneCommand` fields keep these types. The other command payloads are public in `sigil::scene` |
| `FocusScopeKind`, `NodeExtensions`, `ImageCrop`, `icons::IconBaker`, `reactive::DrawPhase` | public only under `kittest` or removed. `AccessibleName` is public in `sigil::node` and `FileDropEvent` in `sigil::events`; `.accessible("text")` and `.file_drop(\|event\| ...)` take them |
| `app::CellWidth`, `terminal::{AfterFrame, EventPump, Placement}`, `profile::{Attribution, PhaseTime, CounterValue}`, `tasks::TaskHandle`, `widgets::{PathFill, TreeRow}` | no public path, or public only under `kittest`; functions return them and fields keep them. `app::Ambiguous`, `terminal::{ClipRoute, Erase, Glyphs, Notices}`, `widgets::{LineId, LineKind}`, and `events::PointerTilt` are public, `ButtonGlyph` and `IconSource` are `widgets::Glyph`, and `PathCmd` is `scene::PathCommand` |
| `style::palette::Palette` | crate-private. Use `Theme` |
| `Cx::scale() -> Scale`, `Cx::set_scale(Scale)` | `Cx::scale() -> f32`, `Cx::set_scale(f32)`. `geometry::Scale` stays public |
| `widgets::ColumnLayout`, `TableState::column_layout` | `TableState::hide_column`, `show_column`, `visible_columns`, `column_width`; the layout is crate-private |
| `Shadow::kind`, `DrawRect::stroke` fields | `Shadow::shadow_kind()`, `DrawRect::stroke()` |
| `reactive::{Runtime, next_interval_deadline, debug}`, `style::unknown_class_warning_count`, `scene::{paint_backdrop, shape_glyph_run}`, `terminal::{map_key, map_mouse, cell_point}`, `A11yProps`, `Cx::new_node`, `clipboard::NoopClipboard`, `Shadow::with_kind` | public only under `kittest`; `with_kind` is `kind` |
| `style::{variant, Variant, is_typography_atom}`, `Theme::from_palette`, `terminal::{install_clipboard, blocks_window, png_window, sextants_window, detect_mux}`, `widgets::{listbox, ListModel, language_for_path, text_runs, ToastEntry}`, `app::{cell_text_lines, ellipsize_cells, ellipsize_cells_start, fit_cells}`, `tasks::panic_message`, `style::font_family_hint` | no public path; `Caps` reports the multiplexer and `ToastQueue` owns its entries |
| `widgets::to_hex(color)` | `Color::to_hex(self)` |
| `Node::a11y.text_cursor` | removed; the `a11y` field is public only under `kittest` |
| `FocusRing::{new, static_visible, theme_color}` | `FocusRing::default()` or `FocusRing::from(visible)` with the `color`, `width`, `offset`, and `opacity` setters; `Widget::focus_ring` takes `impl Into<FocusRing>`, so `focus_ring(signal(true))` is a fixed ring |
| `match_status`, `StatusView` | `show` on `Task::status` |
| `events::request_reveal`, `app::{window_focused, window_occluded}`, `widgets::TreeKey` | removed. No caller used the first two. `Window::focused` and `Window::occluded` stay. `TreeKey` is `Clone + Eq + Hash + 'static` |
| `app::{current_window, use_system_fonts, add_fallback_font}` | crate-private; `AppConfig::font_fallback` adds a fallback face |
| `tasks::installed`, `app::web_fallbacks`, `kernel::NodeType`, `Node::kind`, `terminal::{glyphs, base64, png, blocks, sextants, detect}`, `profile::summary` | public only under `kittest`. `Caps` reports the glyphs and protocol. `scene::Transform` is public |
| `icons::builtin`, `events::PRIMARY`, `style::{DISABLED_OPACITY, ICON_SIZE, OVERLAY_SHADOW}`, `widgets::{DRAG_THRESHOLD_PX, DROP_TARGET_CLASS, MAX_VISIBLE, TOAST_TTL, ChartMark}`, `profile::{count, gauge, enabled, Category}`, `Phase::category` | crate-private. `Shortcut::primary`, `Metrics::icon_size`, and `ChartBuilder::{bar, line, candles}` do those jobs; `profile::count` is public again |
| `profile::Frame` as a nameable path | public only under `kittest`; the functions that return it still work. `profile::Span` and `reactive::EventChannel` with `new` and `emit` are public |
| `Menu::install(self)`, `Tray::install(self)` | both return `AppResult<()>`. `Menu::install` fails on a platform with no native menu bar, and `Tray::install` fails on an invalid icon image. A later native failure is logged. A shortcut key with no native mapping keeps its item without the accelerator and logs a warning. `F13` to `F24` and the other keys `muda` supports now map |
| `Golden` paths from the compile-time `CARGO_MANIFEST_DIR` of the test crate | `Golden::new` reads `CARGO_MANIFEST_DIR` when the test runs. `cargo test` sets it. A test binary run directly must set it, or the path is relative to the current directory |
| `FocusScopeKind::Inert` under `kittest` | removed; nothing used it |
| `CellRect`, `ComponentId`, `FontFamilyHint`, `GradientStopKey`, `PaintFill`, `SpriteKind`, `SpriteSource`, `StateSlot`, `TokenTable`, `ToneContext`, `VariantTokens`, `AnimatorOverlay`, `Asymmetric`, `AttachedAnimator`, `AttachedAnimators`, `AttachedColorAnimator`, `BoundsReactive`, `DecorationGeometry`, `DeltaKind`, `EmitDiff`, `InteractionState`, `LayoutEngine`, `LayoutResult`, `Misalignment`, `MisalignmentKind`, `NodeLabel`, `NodeTag`, `OwnedTextEngine`, `Paragraph`, `ParagraphMetrics`, `ReactivePosition`, `StyleOverlay`, `StyleReactiveEffect`, `TextEngine`, `Tolerance` | crate-private or public only under `kittest`; no caller's non-test code named them. `CellBorder`, `DecorationKind`, `PatternKind`, `ShadowKind`, and `TextFlow` are public in `sigil::scene`, and `ColorScale`, `DateOrder`, `GradientAxis`, `TextDecoration`, `TextFace`, `TextLineHeight`, `TextRun`, and `TextRuns` in `sigil::style` |
| `icons::bake`, `icons::BakedIcon`, `icons::QuadCurve`, `icons::cubic_to_quadratics`, `renderer::scenes::BUILTIN_ICON_ATLAS_BASE_ID` (public only under `kittest`) | removed; `icon(Icon::*)` and `icon_svg` draw paths |
| `SceneCommand::PushClip(Rect)` | `SceneCommand::PushClip(Clip)`; pass `rect.into()` |
| `const DrawGradient::new(rect, LinearGradient)` | `DrawGradient::new(rect, impl Into<Gradient>)`, not `const`; a `LinearGradient` converts with `From` |
| `LayerScope { clip, clip_radii, effect, opacity }` | add `blend: BlendMode::Normal`, or use `..` with a base scope |
| exhaustive `match` on `LayerEffect` | add arms for `Projected`, `Mask` and `Filter` |
| `LayerEffect::LayerBlur { sigma }` | `LayerEffect::Filter(Arc::new([Filter::Blur(sigma)]))` |
| exhaustive `match` on `BlendMode` or `Filter` | add a `_` arm; both are `#[non_exhaustive]` |
| `NodeBuilder::class`, `TextBuilder::class` (feature `kittest`) | `Widget::class("...")`, public on every builder; it takes a `&'static str` of Tailwind classes |
| `style::ClassSource` | crate-private; `class` takes the class string |
| `.transition()`, `.transition_all()`, `.transition_colors()`, `.transition_opacity()`, `.transition_transform()` | `.transition(Transition::new())`, `Transition::all()`, `Transition::colors()`, `Transition::opacity()`, `Transition::transform()`, from `sigil::animation` |
| `.duration(d)`, `.ease(e)`, `.delay(d)` on `Paint` | setters on the value: `.transition(Transition::new().duration(d).ease(e).delay(d))` |
| `Paint::transition_preset(id)` (feature `kittest`) | `.transition(id)`; a `TransitionId` converts with `From` |
| `Widget::animate_opacity`, `animate_radius`, `animate_font_size`, `animate_line_height`, `animate_gap`, `animate_padding_x`, `animate_padding_y`, `animate_width`, `animate_height`, `animate_translate_x`, `animate_translate_y`, `animate_scale_x`, `animate_scale_y`, `animate_rotation` | `Widget::drive(Property::X, Signal<f32>)`, for example `.drive(Property::Height, value)`; each variant documents its unit, and `Opacity` now takes 0.0 to 1.0 where `animate_opacity` took 0 to 10,000 |
| `.bg_linear(axis)`, `.bg_radial(radial)`, `.bg_conic(conic)` | `.background(Background::linear(axis))`, `.background(Background::radial(radial))`, `.background(Background::conic(conic))`, from `sigil::style` |
| `.gradient_from(c)`, `.gradient_via(c)`, `.gradient_to(c)` | `start(c)`, `middle(c)`, `end(c)` on the `Gradient` that `Background::linear`, `radial`, or `conic` returns; a state closure passes the whole gradient again |
| `.gradient_from_position(p)`, `.gradient_via_position(p)`, `.gradient_to_position(p)` | `start_at(p)`, `middle_at(p)`, `end_at(p)` on the same `Gradient` |
| `.bg(color)` with `.bg_linear(axis)` to paint a color under a gradient | `.background(Background::linear(axis).over(color))`; each `background` call replaces the whole background, color and gradient |
| a `.duration(d)`, `.ease(e)`, or `.delay(d)` that outlived a later `.transition()` | each `transition` call replaces the whole transition; a value without a setter uses the timing of its preset |
| exhaustive `match` on `PaintAtom` (feature `kittest`) | add arms for `NoGradient`, `NoStop` and `NoColor` |
| `Paint::background_gradient(BgGradient)` (feature `kittest`) | `.background(gradient)`; a `BgGradient` converts with `From` |
| `Paint::bg(impl Into<ColorValue>)` | `Paint::background(impl Into<Background>)`; `Token`, `Color`, and `ColorValue` convert with `From` |
| `sigil_sign::SignError`, `sigil_sign::PackError` | `sigil_sign::Error`; branch on `error.kind()`, `error.is_password_required()`, `error.path()` |
| `sigil_sign::pack_cli` returning `Result<(), String>` | `Result<(), sigil_sign::Error>` |
| `AppBundle { .. }`, `ResourceCopy { .. }`, `CodeSign { .. }`, `Dmg { .. }` struct literals | `AppBundle::new(name, executable)`, `ResourceCopy::new(from, to)`, `CodeSign::default()`, `Dmg::new(app, volume)` plus setters named for the fields |
| `sigil_kittest::Error::{NoAdapter, Device, Layout, Render, Readback, Unsettled}` | `error.kind() == ErrorKind::..`, `error.is_no_adapter()`, `error.is_unsettled()` |
| `Harness` error type `sigil::Error` | `sigil_kittest::Error` |
| `gpu::Capture { .. }` | `Capture::new(viewport, size_px).scale(..).clear(..)` |
| `GeneratedScene` fields | getters of the same names |
| `Host::route(event)` returning `()` | returns `bool`: whether a key press was consumed |
| `esc_exits(true)`: Escape exited before the UI saw it | exits only when nothing consumed the press |
| terminal `quit_on_ctrl_c(true)`: Ctrl+C exited before routing | copies a selection or reaches a focused `terminal()` first |
| `Metrics::padding(size)`, `Metrics::gap(size)` | `Metrics::control_padding(size)`, `Metrics::control_gap(size)` |
| `Theme::padding(size)`, `Theme::gap(size)` | `Theme::control_padding(size)`, `Theme::control_gap(size)` |
| exhaustive `match` on `Easing`, `HostMode`, `ObjectFit`, `Dimension`, `Display`, `Position`, `TrackSize`, `SceneCommand`, `Clip`, `Elevation`, `Radius`, `ControlSize`, `ClipShape`, `ColorValue`, `MaskImage`, `FontFamily`, `WebFontScript`, `Change`, `Side`, `RowTone`, `SparkMark`, `SparkFill`, `ColWidth`, `ColAlign`, `Density`, `SubmitKey`, `ScrollAnchor`, `SelectionMode`, or terminal `ColorDepth`, `Keyboard`, `Borders`, `Cursor`, `Notify`, `MouseTracking`, `Activity`, `Underline` | add a `_` arm; all are `#[non_exhaustive]` |
| under feature `kittest`: crate-root `NodeType`, `AccessibleName`, `FocusScopeKind`, `ImageCrop`, `NodeExtensions`, `CaretGeometry`, `TextMeasure`, `HitPosition`, `TextAffinity` (use `sigil::node::AccessibleName`, `sigil::text`, and `sigil::kernel` under `kittest`); `animation::{KeyframesId, linear_channel_to_srgb}`, `style::{BorderStyle, GradientShape, FilterAtom, MaskAtom, MillisU16, PaintStyle, SpaceAtom, SpaceStyle, Backface, StopSlot, text_style_for}`, `terminal::{base64, detect}`, `widgets::interaction`, `TextBuilder::runs`, `Date::parse_in`, `Date::render_in` | crate-private |
| `Dimension` | `Length` |
| `PathNode::fill` and `stroke` took an unnameable `PathColor` | `PathBuilder::fill_color` and `stroke` take `impl Into<ColorValue>`: pass a `Color`, a `Token` or a `ColorValue` |
| `Rgba8` | `Color` |
| `ColorValue::Rgba8(..)` | `ColorValue::Color(..)` |
| `sigil::style::Color` (f32 RGBA) | `sigil::style::Rgba`, public only under `kittest`, `#[doc(hidden)]`; app code uses `Color` or `ColorValue` |
| `StateVariants` | `States` |
| `FlawKind` (the rule passed to `allow_flaw`) | `Flaw` |
| `Flaw` (the reported record from `flaws()`) | `Issue` |
| `events::GestureKind` | `events::Gesture` |
| `widgets::ChangeKind` | `widgets::Change` |
| `profile::BackendKind` | `profile::Backend` |
| `widgets::DiagSpan` | `widgets::Diagnostic` |
| `on_gesture`, `on_ime`, `on_paste`, `on_scroll`, `on_selection_copy` | `gesture`, `ime`, `paste`, `scroll`, `selection_copy` |
| `Paint::bg`, `Paint::border_w`, `Paint::rounded` | `Paint::background`, `Paint::border_width`, `Paint::radius` |
| `Tone::Warn`, `Token::Warn1`..`Token::Warn12` | `Tone::Warning`, `Token::Warning1`..`Token::Warning12`; class strings keep `warn-*` |
| `Display` in `sigil::prelude` | `sigil::layout::Display`; the prelude no longer shadows `std::fmt::Display` |
| `FocusIntent`, `ColAlign`, `ColWidth`, `DiagSeverity`, `PointerKind`, `terminal::Caps` | `FocusDirection`, `ColumnAlign`, `ColumnWidth`, `Severity`, `PointerType`, `terminal::Capabilities` |
| `container().layout(LayoutStyle::flex_column())` | `column().gap(Space::None)`. `column()` alone adds the theme gap and takes the width its parent gives it |
| `container().layout(LayoutStyle::flex_row())` | `row().gap(Space::None).align_items(AlignItems::Stretch)`. `row()` alone adds the theme gap, centres its children, and takes the width its parent gives it |
| `container().layout(LayoutStyle::grid(columns))` | `grid(columns)` |
| `container().layout(LayoutStyle::block())` | `container()` |
| `container().layout(LayoutStyle::pane(w, h))` | `column().width(w).height(h).gap(Space::Xs)` |
| `.layout(LayoutStyle::default().gap(space))`, and the same for `padding`, `padding_x`, `margin`, `fill`, `fill_width`, `height`, `min_width`, `max_width`, `shrink`, `flex_basis`, `align_items`, `align_self`, `justify_content`, `flex_wrap`, `display`, `position`, `inset_top`, `overflow`, `grid_columns`, and each other `LayoutStyle` setter | the `Widget` setter of the same name: `.gap(space)` |
| `.layout(LayoutStyle::default().fixed_size(w, h))`, `.min_size(w, h)`, `.max_size(w, h)` | `.width(w).height(h)`, `.min_width(w).min_height(h)`, `.max_width(w).max_height(h)` |
| `.layout(LayoutStyle::default().width(px))` | `.width(px)`, which also lowers a larger minimum width |
| `.layout(LayoutStyle::default().grow(factor))` | `.grow(factor)`, which keeps a minimum size that the node already has |
| `Widget::width(impl IntoPx)`, `Widget::height(impl IntoPx)` | `Widget::width(impl Into<Length>)`, `Widget::height(impl Into<Length>)`. `Length::Percent`, `Length::Fill`, and `Length::Auto` keep their meaning, where each was 0 px; `rows`, `cols`, and `em` resolve at layout |
| a node with no `LayoutStyle` constructor added padding and border to its size: `container().padding(Space::Lg).width(300.0)` was 332 px wide | a size includes padding and border on every node: the same chain is 300 px wide |
| `style::grid(cell)` | `style::cell(cell)` |
| `ScrollBuilder::fill` | `Widget::fill`, with the same effect. `.fill_height()`, `.height(Length::Percent(..))`, `.min_height(..)`, and `.max_height(..)` on a scroll area now also set the height that the scroll range uses |
| `menu_bar().fill()` | `menu_bar().fill_width()` |
| `sparkline(..).fill(SparkFill)` | `sparkline(..).area(SparkFill)` |
| a layout setter on a splitter pane, such as `.pane(node).width(180.0)`, set the splitter | it sets the pane. `.width(px)` on a pane of a horizontal splitter and `.height(px)` on a pane of a vertical splitter set its initial size. Call `.commit()` first to set the splitter |
| `ControlSize::Grow` | `ControlSize::Wide` with `.fill_width()` on the control. On `button`, `Grow` used the `Default` height, so write `.size(ControlSize::Default).fill_width()` there |
| `VirtualListBuilder::viewport_fill()` | `.fill()`. `.fill_height()` or a percent height on a virtual list also makes it take the height of its parent, grow, and measure its viewport |
| `TableBuilder::viewport_fill()` | `.fill_height()`. A percent height on a table does the same |
| `PaneBuilder::initial(px)` | `.width(px)` on a pane of a horizontal splitter, `.height(px)` on a pane of a vertical splitter |
| `PaneBuilder::min(px)`, `PaneBuilder::max(px)` | `.min_width(px)` and `.max_width(px)` on a pane of a horizontal splitter, `.min_height(px)` and `.max_height(px)` on a pane of a vertical splitter. They clamp the drag and also set the pane's own limits |
| a paint, event, or `map_node` setter after `.pane(node)`, such as `.pane(node).background(..)`, set the splitter | it sets the pane. Call it before the first `.pane(..)` or after `.commit()` to set the splitter |
| `Paint::bg_clip_text()`, `Paint::bg_image(image)` | `Paint::background_clip_text()`, `Paint::background_image(image)`. The class string `bg-clip-text` is unchanged |
| `ScaleRole::Warn` | `Tone::Warning` |
| `ButtonBuilder::shortcut(text)`, `MenuItem::shortcut(text)`, the `MenuItem::shortcut` field | `ButtonBuilder::hint(text)`, `MenuItem::hint(text)`; `Widget::shortcut(shortcut, handler)` still binds a key |
| `divider_horizontal()`, `divider_vertical()` | `divider(Orientation::Horizontal)`, `divider(Orientation::Vertical)` |
| `Paint::line_height_px(px)`, `Paint::letter_spacing_em(em)` | `Paint::line_height(px)`, `Paint::letter_spacing(em)`; the units are unchanged |
| `Paint::mix_blend(BlendMode)` | `Paint::blend(BlendMode)` |
| `ColorValue` in `sigil::prelude` | `sigil::style::ColorValue`; setters still take `Token` and `Color` through `Into<ColorValue>` |
| `set_theme(theme)`, `sigil::app::set_theme(theme)` | `theme.apply()` |
| `NodeBuilder::owns` | removed |
| `TextCursor` in `sigil::prelude` | crate-private; `NodeBuilder::text_cursor(caret, anchor)` stays |
| `Status<T, E = Box<dyn Error + Send + Sync>>` | `Status<T, E = sigil::Error>` |
| `Frame` fields (`exit`, `dirty`, `animating`, `intervals`, `layout`) | getters: `exit()`, `dirty()`, `animating()`, `intervals()`, `layout()` |
| `Track` and `Notification` public fields | builder setters on `Track::new(title)` and `Notification::new(title, body)` |
| `WindowCommand::{SetZoom, SetTitle, SetPosition, SetLevel}` | `WindowCommand::{Zoom, Title, Position, Level}` |
| `Theme::{gray, accent_scale, success, warn, error, info}` | `Theme::tone(Tone, ColorScale)` |
| `Terminal::{progress, bell, activity}(Option<...>)` | take `impl Into<Option<...>>`; `Some(x)`, bare `x`, or `None` compile |
| `sigil_kittest::parity::Frame { ... }`, `Diff.mean`, `Diff.close` | `Frame::new(scene, viewport, size, clear)`, `Diff::mean()`, `Diff::close()` |
| `ScrollBuilder::scroll_bind`, `VirtualListBuilder::scroll_bind`, `TableBuilder::scroll_bind` | `scroll_y` |
| `VirtualListBuilder::{scroll_x_bind, content_height_bind, max_scroll_bind}` | `scroll_x`, `content_height`, `max_scroll` |
| `SplitterBuilder::{sizes_bind, settle_bind}` | `sizes`, `settled` |
| `VirtualListBuilder::shrink_to_content` | `fit` |
| `TableState::toggle_sort(col)`, `TableState::cycle_sort()` | `TableState::cycle_sort(col)`, `TableState::next_sort()` |
| `DiffSelection::select_line(patch, id, extend)` | `DiffSelection::select_line(id)` and `DiffSelection::extend_line(patch, id)` |
| `LineId` fields (`hunk`, `line`) | `LineId::new(hunk, line)`, `hunk()`, `line()` |
| `FuzzyMatch` fields (`score`, `positions`) | `score()`, `positions()` |
| `ListState` fields (`cursor`, `scroll`, `expanded`, `query`, `caret`), `select`, `toggle` | getters `cursor()`, `scroll()`, `expanded()`, `query()`, `caret()`; `move_to`; `toggle_expanded` |
| `TextEditBuilder::caret_visible(&mut self)`, `TextFieldBuilder::{caret_visible(&mut self), scroll_offset(&mut self)}` | chaining setters taking `Signal<_>` and returning `Self` |
| `Token::Gray1..Info12`, `Tone::token(step)`, `Token::scale` | `Tone::step(step)` / `Token::Step(tone, step)` |
| `text_edit(v)`, `TextEditBuilder`, `.caret_position(s)` | `text_field(v).multiline()`, `TextFieldBuilder`, `.caret(s)` |
| `TextEditState`, `state.select(b, extend)`, `.state(s)` | `TextState`, `state.select(b)` / `state.extend(b)` / `state.clear_selection()`, `text_field(state)` |
| `VirtualListBuilder::selection_state()`, `DiffBuilder::select(s)` | `VirtualListBuilder::selected(s)`, `DiffBuilder::selected(s)` |
| `Host::emit(viewport)` | `Host::scene(viewport)` |
| `GlyphRun` `u16` basis-point opacity + `rect`/`node` fields | `f32` (`0.0..=1.0`) opacity + `rect()`/`node()` getters |
| `terminal::Lines` + `Cell` public fields | `Cell` getters (`fg()`, `bg()`, `bold()`, `italic()`, `dim()`, `underline()`, `strike()`, `underline_ink()`) |
| Public fields on event structs, `Shadow`, `Locale`, `TextStyle`, `TextFace`, `TextRun`, `TextRuns` | getters and builder setters |

### Changed

- `panel` and `PanelBuilder` are removed; `card` takes a title. Migration: `panel().title("T").child(body)` becomes `card(body).title("T")`, and `.elevation(0)` becomes `.layer(Layer::Page)`. A card has 12 px padding where a panel had 16 px; chain `.layout(LayoutStyle::default().padding(Space::Lg))` to keep it.
- Names shortened to one word each. Migration: `derived(f)` becomes `memo(f)`; `IntervalId` becomes `Timer`; `ModifiersState` becomes `Modifiers` and `ModifiersState::CONTROL` becomes `Modifiers::CTRL`; `.on_added(f)` becomes `.added(f)`; `.animate(keyframes)` becomes `.animation(keyframes)`; `signal_writer(s)` becomes `s.writer()`.
- Setters and handlers are the field or event name with no prefix. Migration: `on_cleanup(f)` becomes `cleanup(f)`; `AppConfig::on_close`/`on_exit` become `close`/`exit`; `esc_exits`/`quit_on_ctrl_c` become `exit_on_esc`/`exit_on_ctrl_c`; `request_exit()` becomes `exit()`; `Locale::with_date`/`with_group_size`/`with_zero` become `date`/`group`/`zero`; `terminal::set_title`/`set_mouse_tracking` become `title`/`mouse_tracking`; `Screen::set_access`/`set_dt` become `access`/`dt`; `AsyncRuntime::set_waker` becomes `waker`.
- More prefixes dropped. Migration: `request_focus(id)` becomes `id.focus()`; `tasks::on_message` becomes `tasks::listen`; `SceneBuilder::with_clip`/`with_transform`/`with_layer`/`with_composite_layer` become `clip`/`transform`/`layer`/`composite`; `Shadow::with_kind` becomes `kind`; `with_on_solid` becomes `on_solid`; `Node::get_extension`/`get_extension_mut` become `extension`/`extension_mut`.
- One setter and one getter per job. Migration: `Harness::get_by_role(role, name)` becomes `get((role, name))` and `get_by_text(text)` becomes `get(text)`; `get` also still takes a `Query`. `StateStyle::with_hovered(atoms)` and the other four become `slot(StateSlot::Hovered, atoms)`, and `hovered()` and the other four readers become `get(StateSlot::Hovered)`. The layout constraint `with_fixed_width(w)` becomes `width(w)`; `with_min_content_width` and `with_max_content_width` had no callers and are removed.
- Setters and constructors lose their prefix where no getter has the name. Migration: `events::request_window_command(command)` becomes `events::window_command(command)`, `Cx::add_child(parent, child)` becomes `Cx::append(parent, child)`, `LayoutEngine::with_text_measure(measure)` becomes `LayoutEngine::new(measure)`, `app::set_ambiguous(width)` becomes `app::ambiguous(width)`, `layout::set_text_geometry(measure)` becomes `layout::text_geometry(measure)`, `testing::with_cx` and `with_runtime` become `testing::cx` and `testing::runtime`, and `Screen::with_caps(cols, rows, caps)` becomes `Screen::new(cols, rows)` followed by `set_caps(caps)`. A setter keeps `set_` only where a getter of the same name exists, such as `Cell::text` and `Cell::set_text`.
- One focus ring setter. Migration: `focus_ring_color(c)`, `focus_ring_width(w)`, `focus_ring_offset(o)` and `focus_ring_opacity(a)` become `focus_ring(FocusRing::default().color(c).width(w).offset(o).opacity(a))`; `focus_ring(visible)` still takes a `Signal<bool>`, and `FocusRing::from(visible)` adds the other settings to it. `FocusRing` is in `sigil::style`.
- Two widget methods take one-word names. Migration: `presence_tween(shown, ms, easing)` becomes `presence(shown, ms, easing)` and `selectable_text(yes)` becomes `selectable(yes)`.
- One window type and one way to command it. Migration: `WindowId` becomes `Window`; `events::window_command(command)` (0.1 `request_window_command`) is removed, use `Window::current()` and `Window::command(command)`; `open(url)` becomes `open_url(url)`.
- `Size` is a width and a height. Migration: the layout constraint `Size` (`Size::fixed(w, h)`, `geometry::Size`) becomes `Constraint`, and `LogicalSize` becomes `Size`. The prelude exports both.
- `DiffBuilder::change` is removed: the widget stored the callback and never called it. Switch layouts by passing a different `DiffView` to `.view(..)`.
- Doc comments on about 700 public items now say what the item does, its unit and its default, in place of sentences generated from the name.
- Three names now say what the code does. Migration: `TooltipBuilder::clamp_top()` becomes `below()`; `VirtualListBuilder::scroll_into_view(index)` becomes `initial_scroll(index)`, which takes a `ScrollAnchor` or a row index; `Locale::group(size)` becomes `group_size(size)`, because the field `group` is the separator.
- Key names follow the web: `Key` (the named keys) becomes `NamedKey`, and `LogicalKey` becomes `Key`, so `LogicalKey::Named(Key::Enter)` is now `Key::Named(NamedKey::Enter)`. `.invalid_when(flag)` becomes `.invalid(flag)` and `.visible_when(flag)` becomes `.visible(flag)`.
- `Error::new` takes the kind: `Error::with_kind(kind, err)` becomes `Error::new(kind, err)`, and `Error::new(err)` becomes `Error::new(ErrorKind::Other, err)`. The unused `ErrorKind::Svg` is removed.
- `button` takes a text label or an `Icon`, and `icon_button` is removed. Migration: `icon_button(Icon::Close, f)` becomes `button(Icon::Close).activate(f)`; `icon_button("×", f)` becomes `button("×").ghost().square().activate(f)`. `ButtonBuilder::square` is new.
- `sheet` is removed. Migration: `sheet(show)` becomes `modal(show).side(Side::Right).dismiss_on_backdrop(true)`.
- The `icon_svg` function is removed. Migration: `icon_svg(source)` becomes `icon(Glyph::svg(source))`. `Glyph` has a new `Svg` variant and is in `sigil::widgets`.
- Two `sigil_kittest::Harness` values on one thread each get the results of their own tasks. Before, the second harness took the thread's task dispatcher and the first one's `spawn`, `task` and `signal_writer` results never arrived.
- A `SignalWriter` writes for as long as its signal is alive, and `Signal::writer` works outside a reactive scope. Before, a writer made in a scope that later ended dropped every write without an error, and a write to a disposed signal panicked the UI thread; that write is now dropped.
- A scroll moves one shared offset that `LayoutResult::rect` adds when read; it no longer rewrites the layout of every node under a `scroll`, a `virtual_list` or a `position_reactive` node. `rect` returns the same on-screen rectangle as before. `LayoutResult::rect` is no longer `const`, and `ReactivePosition` is `Clone` but no longer `Copy`.
- A node gets its own reactive owner only when its build asks for the current owner, or when the node has event handlers or a style binding. Before, every node got one. A node without its own owner runs later callbacks under the nearest owner above it. Each row of a reactive child list still gets an owner.
- `Host::frame_due()` returns whether a frame has work to do without new input: a signal write, an animation, a clock, or a timer that is due. An embed host calls it to skip `Host::frame` while the UI is idle.
- `terminal::Cell` is `Copy` and holds its text and hyperlink in 30 bytes. Text longer than seven bytes and each hyperlink are stored once for the life of the process, up to 65,534 of each and 2,048 bytes per entry; past that a cell shows U+FFFD and a link is dropped.
- `A11yProps` keeps `role`, `name`, `focusable`, `traps_tab`, `hittable` and `focus_scope`; its other fields moved to `A11yDetail`, which `A11yProps` dereferences to, so `props.checked` still reads and assigns. Migration: build one with `A11yProps::default()` and assign the fields; a struct literal can no longer name the moved fields. The `NodeBuilder` setters for the moved fields are no longer `const fn`.
- The minimum supported Rust version is 1.96. Before, it was 1.95.
- `Combobox::placeholder` and `TagInputBuilder::placeholder` take `impl Into<String>`. Before, they took `&'static str`.
- `TextFieldBuilder::change`, `submit` and `blur` add a callback on each call. Before, a second call replaced the first, so `number_field(..).text_field().submit(f)` dropped the clamp on commit.
- `range_slider` returns `SliderBuilder<RangeInclusive<f32>>`, and its `.change` gets the whole range after either thumb moves. Before, it got one thumb's value.
- `Theme::token(Token, Color)` sets a theme token, with `Token::Custom(name)` for an app token. It replaces `Theme::register_token` and `Theme::with_token`.
- A table cell edit commits and closes when focus leaves the editor. Before, it stayed open.
- `Theme::tone(Tone, ColorScale)` replaces `with_gray_scale`, `with_accent_scale`, `with_success_scale`, `with_warn_scale`, `with_error_scale`, and `with_info_scale`. `with_direction`, `with_locale` and `with_strings` become `direction`, `locale` and `strings`.
- `Theme::color` takes a `Token`, a `Color` or a `ColorValue`, and replaces `Theme::resolve_color`. `Theme::text_muted`, `surface_subtle` and `border_strong` are removed; read `Theme::color(Token::OnSurfaceMuted)`, `Token::SurfaceSubtle` and `Token::BorderStrong`, which also follow token overrides.
- A `combobox` keeps focus in its input after the user picks an option. Before, a pick rebuilt the input and focus moved to the next control.
- A parsed field (`date_field`, `time_field`, `number_field`, `color_field`) and a `text_field` with `invalid_when` report the invalid state to screen readers, not only in paint. `field` no longer clears it when there is no `.error`, and `invalid_when` adds a condition on each call instead of replacing the last one.
- The clear button, the tag remove button, the range slider thumbs and the 12-hour day periods read their text from `StringKey::{Clear, RemoveItem, Minimum, Maximum, Am, Pm}`, so a `Strings` table can translate them. `Locale::format_time` and `parse_time` take the `Strings` table.
- `DateFieldBuilder` implements `FieldControl`, so `field(..).error(..)` accepts it.
- `Locale` has a new `hour12` field. Migration: a `Locale { .. }` struct literal adds `hour12: false` for a 24-hour clock.
- `virtual_list` with a fixed `viewport_height` and `follow_end`, `initial_scroll` or a scroll to a row no longer paints a blank viewport on first frame.
- `open` and `link` URLs work in terminal-only builds (feature `terminal`).
- Parsed fields (`date_field`, `number_field`, `color_field`, `time_field`) no longer reformat the text while the user types; before, a date field turned "2026-08-1" into "2026-08-01" under the caret. The text is rewritten to the stored value when the user presses Enter or leaves the field, when the app sets the value, and when the locale changes, so "500" in a `0..=100` field shows "100" and "1.50" shows "1.5".
- `splitter` and `tabs` free their drag state when they unmount. Before, each mount leaked it.
- `AppConfig::exit` runs on the terminal host and in builds without the `desktop` feature. Before, the terminal host never called it.
- A text selection belongs to the host where it was made. Before, a frame in a second window or host could clear it or highlight text in the wrong tree.
- Engine internals left the public API. `Cx::dirty_all`, `has_specific_dirty`, `get_mut_untracked`, `instance_id`, `root_owner`, `roots`, `host_mode` and `describe`, `Node::attach_untracked`, `get_extension_mut_untracked` and the `owner` field, `PointerEvent::set_captured` and `capture_requested`, `SceneBuilder::draw_glyph_run_for_node` and `style::cell` are crate-private. `Cx::dirty`, `clear_dirty`, `mark_mutated`, `mutation_generation`, `scale`, `set_scale`, `first_child`, `flush_announcements`, `flush_reactive_children`, `dump_tree`, `Node::version`, `Host::set_hyperlinks`, `reset_emit`, `paint_cells` and `SceneBuilder::pop_layer` are public only under feature `kittest`. `Host::take_exit`, `Host::hit_path` and `Node::text_cursor` are removed; a frame's exit request is `FrameOutput::exit`.
- Every type that a public function takes or returns can be named. `sigil::widgets` exports each widget builder and its value types (`Glyph`, `FuzzyMatch`, `LineId`, `LineKind`, `DiffSource`, `FieldControl`, `IntoList`, `NumericKind`). `sigil::style` exports `ColorScale`, `Locale`, `DateOrder`, `Strings`, `StringKey`, `TextLineHeight`, `TextDecoration`, `TextRuns`, `TextRun`, `TextFace` and `StateStyleBuilder`. `sigil::scene` exports `SceneBuilder`, `CommandScene`, `DrawRect`, `GlyphRun`, `Wash`, `StrokeCap`, `StrokeJoin`, `BlendMode`, `Shadow`, and the command payload types. `sigil::geometry` exports `Scale`. `Frame`, `FileDropEvent`, `PointerTilt`, `MessageReceiver`, `Span`, `Subscription` and the terminal `ClipRoute`, `Erase`, `Notices` and `Glyphs` are public. `Frame` and `FileDropEvent` are `#[non_exhaustive]`.
- Every builder type is `#[must_use]`.
- `ButtonGlyph`, `IconSource` and `IntoIconSource` are one type, `widgets::Glyph` (`#[non_exhaustive]`, variants `Text`, `Icon` and `Svg`). `icon`, `button` and `TreeNode::leading` take `impl Into<Glyph>`, so the same values work.
- `ScrollBuilder::extent` takes `impl Reactive<f32>`: a number, a `Signal<f32>`, or a closure. `IntoScrollExtent` is removed.
- `GlyphRun` no longer derefs to `SharedGlyph`. Read a run with `text()`, `color()`, `style()`, `opacity()` and `tone()`. `SharedGlyph` is crate-private. Migration: `run.text.clone()` becomes `run.text().to_owned()`.
- `Theme::tokens` is crate-private, and `TokenTable` and `TokenLayer` are no longer exported. Read a token with `Theme::color` and set one with `Theme::token`. Writing the field directly left `accent` and its scale stale. A `Theme` is no longer built with `..Theme::default()` outside the crate; start from `Theme::default()` and set fields.
- `app::FrameInputHook`, `FrameLayoutHook` and `FramePaintHook` are public, like `FrameHooks` that names them. `render_style_for` (feature `kittest`) no longer takes an unused node id.
- `terminal::Capabilities` and `terminal::Notices` fields are private. Read them with getters of the same names, for example `caps.backdrop()`. `Screen::with_caps` and `Screen::set_caps` are public only under feature `kittest`; a host gets its capabilities from `Capabilities::detect` or `probe`. `terminal::ClipRoute` and `terminal::Fills` are `#[non_exhaustive]`.
- `StyleAtom` and its payload types, `emit_scene`, and the terminal `map_key` and `map_mouse` are public only under feature `kittest`.
- `kernel::Patch` is renamed `NodePatch`. `LayoutStyle::pane`, `StyleAtom::UnknownThemeRef`, `unsupported_style_warning_count_for` and `Hotspot::RowRebuild` are removed. `AppConfig::exit` builds only with the `desktop` or `terminal` feature.
- A signal write runs each effect that depends on it one time, in dependency order. Before, an effect that read two `derived` values ran one time with one new value and one old value, and then again.
- A write outside `batch` follows the `batch` rules. When an effect runs because of a write, the writes it makes run their dependent effects after it returns. Before, they ran inside the write.
- `NodeBuilder` has `.disabled(..)` and `.activate(..)`. Any node is an interactive element: it dims and ignores input when disabled, and runs the action on a primary click, Enter, or Space. `interactive()` and `InteractiveBuilder` are removed. Migration: `interactive(node).hover(..)` becomes `node.hover(..)`; `interactive(node).disabled(d)` becomes `node.disabled(d)`. The control hover fill is no longer implied: add `.hover(|s| s.background(Token::SurfaceHover))`.
- A disabled node runs no pointer press, key, shortcut, or accessibility action handlers. Pointer enter and leave and focus out still run so hover and focus state stay correct. Before, only `.activate` checked the disabled flag.
- A disabled node also runs no `.paste`, `.gesture`, or `.ime` handlers. An input method end still reaches it. A control that becomes busy or disabled during a press releases its pressed state on pointer up and shows no hover or press style while blocked. A radio option is selected only by a release that follows a press on that option. A node that mounts or moves under the pointer shows its hover style on the first frame, with no transition.
- A node with only state styles (`.hover`, `.pressed`, `.disabled`) no longer reports a click action to screen readers.
- A2UI `Markdown`, `Mermaid`, `Diff`, `Chart`, and `Table` pieces bound to a data path update when the data model changes. Before, they showed the value at mount.
- `a2ui` is behind the `a2ui` feature, which is on by default. Migration: a build with `default-features = false` adds `features = ["a2ui"]` to keep `a2ui()`.
- A style change that arrives before a node is first painted takes effect at once, with no transition. On the first frame, the control that gets focus shows its focused style at once. Before, it ran the rest-to-focus transition for about 60 frames.
- `for_each` calls `render` only for a key that is new to the list. Before, it called `render` for every item on each write and dropped the result for the keys that stayed.
- Vega-Lite figures are behind the `vega` feature, which is on by default. `scene` no longer pulls in the QuickJS engine. Migration: a build with `default-features = false` that wants figures adds `features = ["vega"]`.
- The terminal backdrop watcher stops at once when the app exits. Before, exit waited up to 250 ms.
- `sigil-ui` no longer depends on `crossbeam-channel`, and `notify` builds only with the `desktop` or `terminal` feature.
- Vega-Lite charts and SVG icons (`icon(Icon::*)`, `icon(Glyph::svg(..))`) lower to scene commands like Mermaid instead of a CPU bitmap, so no vector widget rasters on the CPU in a window build. `icon_svg` now keeps explicit colors in the SVG; `currentColor` and a root with no `fill` take the tone's color.
- Mermaid, Graphviz and math pictures lower their SVG to scene commands instead of one resvg bitmap: paths with fill rules, caps, joins and dashes, linear and radial gradients with spread methods and transforms, clip paths, masks, group opacity and blend modes, patterns, images, and blur, drop-shadow, saturate and hue-rotate filters. They stay sharp under zoom and past the 4096 px bitmap cap, and a theme change lowers them again instead of rastering. Other filter primitives, focal radial gradients and patterns of more than 4096 tiles keep a resvg bitmap for that subtree only, rastered again at the device size.
- Each SVG picture, vector or bitmap, draws in one layer that composites over its backdrop in sRGB-encoded space, so antialiased edges and translucent ink match resvg over any surface and opaque ink keeps its color.
- An SVG picture lowers to at most 65,536 scene commands and 16,384 nodes, counted at each clip, mask and pattern use; past that it draws as one resvg bitmap. Nested patterns whose copies pass the budget draw the patterned shape as a bitmap.
- `text_shadow` and the `text-shadow-*` classes inherit to descendant text, as CSS `text-shadow` does; a descendant's own text shadow replaces the inherited one, and `text_shadow(None)` or `text-shadow-none` cancels it.
- `math()` typesets with RaTeX and the KaTeX fonts instead of latex-rust: 136 of 138 common constructs draw, up from 115. `\label` and `\iddots` no longer parse and fall back to text.
- Charts pick nice ticks in steps of 1, 2 or 5 × 10^k, include zero on the value axis, label time axes with time ticks, and draw a zero rule; `stacked()` puts the first series on top. Vega-Lite fences sort categories ascending, honor `sort` and `scale.domain`, show axis titles, break lines at nulls, and default to a 300 px plot.
- Color transitions mix in premultiplied gamma-encoded sRGB, as CSS Color 4 specifies for legacy colors, instead of linear light; black to white at 50% is now 128, was 188.
- Percent translate covers ±1600% in steps of 1/2048 of the width, was ±327.67%.
- `Easing::cubic_bezier` with a non-finite control point eases linearly.
- `Easing` has a new variant, so an exhaustive `match` on it needs a new arm.
- `DrawGradient::new` takes `impl Into<Gradient>` and is no longer `const`; `DrawGradient::gradient` is a `Gradient`. A `LinearGradient` converts with `From`.
- Gradients mix their stops in premultiplied gamma-encoded sRGB on the GPU, the CPU renderer and vector paths, as CSS Color 4 specifies for legacy colors; black to white at 50% is 128 on every renderer, was 188 on the GPU and 180 on the CPU.
- A gradient stop placed before an earlier stop moves up to the earlier stop's position, as in CSS Images 4 §3.5.3, instead of the stops being sorted.
- Gradient text draws conic, elliptical radial and repeating gradients.
- A 3D-transformed layer with part of its plane behind the viewer draws the part in front of the viewer, instead of nothing.
- Layout is set with the container functions `row`, `column`, `grid`, and `container` and with the layout setters on `Widget` (`.gap(..)`, `.padding_x(..)`, `.fill()`, `.min_width(..)`, `.align_items(..)`, and the rest). `Widget::layout(LayoutStyle)` applies a whole `sigil::layout::LayoutStyle` at once; the migration table maps each constructor and method.

### Fixed

- Clicking a row of a filtered `command_menu` runs that row; before, the click cleared the filter and ran the row at the same position in the full list.
- A `command_menu` longer than twelve rows scrolls, and the arrow keys keep the selected row in view; before, rows past the window edge could not be reached.
- Every text in `diff` meets 4.5:1 contrast in the light and dark themes: the gutter, file and hunk headers, status row and changed lines use stronger gray and hue steps.
- Dragging a splitter handle or pressing its arrow keys resizes the panes; before, the panes did not move.
- A splitter drag on panes below their minimum sizes no longer panics.
- Drag-selecting text repaints only the frame instead of laying out the whole tree, so each pointer move costs about 0.42 times as much.
- Two hosts on one thread, such as two windows or an app and a test harness, no longer take each other's focus requests, announcements, dialogs, pending theme or open popups when these are made while a host runs.

### Added

- `TableColumn::editable(text, commit)` edits a cell in place. Double-click or Enter starts, Enter commits, Escape cancels, and Tab and Shift-Tab move between editable cells. The result is written into the table's rows, and an open edit follows its row by `Identifiable` id when the app replaces the rows.
- `virtual_list(..).end_reached(rows, f)` and `table(..).end_reached(rows, f)` run `f` once each time the user scrolls within `rows` rows of the end, for load-more paging.
- `link(label)`: focusable text that opens `href` or runs an action, with role link, theme hover and focus styles, and an OSC 8 hyperlink on the terminal host.
- `Log` and `log_view(log)`: an append-only log that drops its oldest lines past a limit and follows the tail until the user scrolls up. `Log::push` splits at `\n` or `\r\n`, adds no empty line for a trailing break, and adds one empty line for empty text.
- `badge`, `skeleton`, `pagination`, `breadcrumbs` and `stepper` patterns. A `badge` at zero takes no space or gap. `pagination` moves the page into range when the page count shrinks.
- `range_slider(value, range)`: two thumbs on one track editing a `Signal<RangeInclusive<f32>>`. The start never passes the end, each thumb takes focus and keys, and a drag moves the nearest thumb. When both thumbs sit on one value, the drag direction picks the thumb.
- `time_field` with `Time` and `DateTime` (both ordered): a time, or a date and time, input with a 12- or 24-hour clock from the new `Locale::hour12` field, and `Locale::format_time` and `parse_time`. `TimeFieldBuilder` implements `FieldControl`.
- `tag_input(selected, options)`: a filterable multi-select whose chosen options show as removable tags. Backspace in an empty input removes the last tag. `.disabled(..)` also disables the remove buttons.
- `widgets::load_image(location)` loads an image from a file path, `file://` URL (including `file://localhost/` and Windows drive paths) or `data:` URI off the UI thread, with a placeholder while loading and an error view with the reason. It takes the layout setters; on a sized axis the image fills the box with `.fit`. `ImageSource::load(location)` returns the load as a `Signal<Status<ImageSource, Error>>`. Loads are cached by location and, for a file, its modified time and size, so a changed file loads again.
- `load_image` and `ImageSource::load` fetch http(s) URLs in a browser (wasm32), and in native builds with the new `image-http` feature through leyline-http, with a 30 s timeout and a 64 MiB body limit. An error status fails with `ErrorKind::Http`. Without the feature, a native build fails the load with `ErrorKind::Fetch`.
- `benches/sparse_edit.rs` measures one edit in a list of 1,000, 3,000, and 10,000 live rows, split into the frame, the scene, and the accessibility tree.
- The recipes have a Lists section on per-row signals.
- `CommandScene::glyph_runs()` returns the text runs of a scene in paint order.
- `CommandScene::find_text` is public without feature `kittest`.
- `Icon::Columns2`, the Lucide `columns-2` icon, for a split-pane control.
- `Session::command` starts a `std::process::Command` in a pseudo-terminal with its arguments, environment and working directory.
- `View` and `view!`: a widget tree in a `.sigil` file that the running app reloads on save, with no rebuild. A mistake keeps the previous screen and shows the line and column.
- `Space`, `TextRole`, and `Layer`: named steps for spacing, type, and depth. `gap`, `padding`, and `margin` setters no longer take a bare number, and `font_size` takes a reason.
- A `Harness` runs the design rules when it is dropped. See `Flaw`, `Issue`, `Harness::assert_no_flaws`, `Harness::unaudited`, and `Widget::allow_flaw`.
- Keyframe animations: `Paint::animation(Keyframes)`, with `Keyframes` stops, per-stop easing, iterations, `reverse` and `alternate`, and the Tailwind `spin`, `ping`, `pulse` and `bounce` presets. `Property` names the animated value.
- CSS-style transitions: `Paint::transition(Transition)`, with `Transition::new`, `all`, `colors`, `opacity` and `transform`, and the `duration`, `ease` and `delay` setters.
- Vector paths draw on the GPU through `vello_gpu` and on the CPU through `vello_cpu` instead of a tiny-skia bitmap. `PathBuilder` gains `even_odd`, `cap(StrokeCap)`, `join(StrokeJoin)` and `dash`; `scene::StrokeCap` and `scene::StrokeJoin` are new.
- Vega-Lite fences in a window render through the official vega 6.4.0 and vega-lite 6.5.0 running in an embedded QuickJS, with a 2 s deadline, a 256 MB heap cap and no network; the native mapping is the fallback and the terminal path. The figure is named by the spec title and described by its axis titles.
- `ChartBuilder::horizontal()` lays bars along rows.
- `Easing::CubicBezier`, `Easing::cubic_bezier` and the `Easing::IN`, `OUT` and `IN_OUT` curves.
- Radial and conic background gradients: `Paint::background(Background::radial(Radial))` with `RadialExtent`, `Paint::bg(Background::conic(Conic))`, `Paint::bg(Background::linear(GradientAxis))`, the `start`, `middle` and `end` stops with positions, and repeating gradients. `scene::Gradient` and `scene::GradientKind` carry linear, radial and conic paints.
- `Paint::bg_clip_text` paints the background gradient through the glyphs; `Paint::bg_image` tiles an image behind the content.
- `Paint::border_dashed`, `border_dotted` and `border_solid`; dashes follow rounded corners.
- `Paint::clip_path(ClipShape)` clips a node, its background and its children to an inset, circle, ellipse or polygon, and pointer hits outside the shape miss.
- Masks on `Paint`: `mask_image(MaskImage)` with linear, radial, conic and image masks, `mask_stops`, `mask_luminance`, `mask_size(MaskSize)`, `mask_position` and `mask_repeat`, and the Tailwind `mask-*` classes for them.
- `Paint::mix_blend(BlendMode)` and the `mix-blend-*` classes.
- Filters on `Paint`: `blur`, `brightness`, `contrast`, `grayscale`, `hue_rotate`, `invert`, `saturate`, `sepia`, `drop_shadow` and `text_shadow`, with the Tailwind `blur-*`, `brightness-*`, `contrast-*`, `grayscale`, `hue-rotate-*`, `invert`, `saturate-*`, `sepia`, `drop-shadow-*` and `text-shadow-*` classes.
- 3D transforms on `Paint`: `perspective`, `perspective_origin`, `rotate_x`, `rotate_y`, `rotate_z`, `translate_z`, `backface_hidden` and `backface_visible`, with `Property::RotateX`, `RotateY` and `TranslateZ` for keyframes.
- Keyframes and transitions animate filters, drop and text shadows, gradient stop colors and positions, mask stops and the 3D turns, depth and perspective. `Property` gains `RotateZ`, `Perspective`, `Blur`, `Brightness`, `Contrast`, `Grayscale`, `HueRotate`, `Invert`, `Saturate`, `Sepia`, `DropShadow`, `TextShadow`, `GradientFrom`, `GradientVia`, `GradientTo`, their `*Position` forms, `MaskFrom` and `MaskTo`. Filter lists, gradients and shadows interpolate as CSS does and jump when they do not match.
- Class strings parse `bg-linear-to-*`, `bg-radial`, `bg-conic-*`, `from-*`, `via-*`, `to-*`, `bg-clip-text`, `border-solid`, `border-dashed` and `border-dotted`.
- `SplitterBuilder::proportional` keeps pane shares when the parent resizes, and `SplitterBuilder::settled` reports the sizes once when a drag or key move ends.
- `Error` predicates `is_io()`, `is_closed()`, `is_config()`, `is_clipboard()`, `is_layout()`, `is_gpu()`, and `is_no_adapter()`; `Status::is_idle()`; and `DiffSelection::clear()`.
- One error type for the crate: `sigil::Error` with `sigil::ErrorKind` replaces 13 module error types. `docs/guide/errors.md` maps each old type to its kind.
- The host traits, the renderer, and the window host are crate-private. The `kittest` feature exposes them to the harness. Text measurement types are `sigil::text`.

#### Core

- A retained widget tree built from component functions whose results implement `IntoNode`. `sigil::run` runs the application on the selected host.
- `Host` embeds the same tree in another engine's event loop, with event routing, layout, drawing, and host-rendered `surface` regions.
- Reactive state through `signal`, `memo`, and `effect`. `batch` groups writes, and `interval` runs periodic callbacks.
- Conditional views through `show` and `switch`, with optional state preservation for hidden views.
- Keyed collections through `for_each`, preserving each item's node and state while its key remains.
- Async work through `task` and `spawn`, with reactive status and cancellation for tasks. `show` on `Task::status` renders loading, result, and error views.
- `tasks::blocking` runs blocking work inside async tasks. `Signal::writer` and `tasks::listen` deliver results from other threads to the UI thread.
- Flexbox and grid layout through `row`, `column`, `grid`, and the `Widget` layout setters, with pixel sizes and text units `rows`, `cols`, and `em`.
- Keyboard, pointer, scroll, paste, and gesture handlers through `Widget`. `Shortcut` parses and displays key combinations, with `Shortcut::primary` selecting Command or Control for the platform.

#### Widgets

- Push buttons through `button`, with independent tone and fill choices, disabled states, and activation callbacks.
- Boolean controls through `checkbox` and `toggle`.
- Single-choice controls through `radio_group` and `segmented`.
- Numeric sliders through `slider`, with step sizes and horizontal or vertical orientation.
- Single-line input through `text_field`, with password masking, leading and trailing content, and a clear button.
- Multi-line input through `text_field(..).multiline()`, with selection, reactive focus, and configurable submission keys.
- Numeric input through `number_field`, with range limits and locale-aware parsing.
- Date input through `date_field`, using the theme's date format.
- Color input through `color_field`, with a swatch and hexadecimal entry.
- Form labels and validation through `field`, with required markers, error messages, and accessible descriptions.
- Choice fields through `dropdown` and searchable `combobox`.
- Sortable, filterable tables through `table` and `TableState`, with row selection and configurable columns.
- `virtual_list` builds only visible rows, supports measured row heights, and can follow a row or the end of the list. Measured lists with stable keys preserve the visible item when rows are inserted above it.
- Explorer and outline views through `tree` and `TreeNode`, with controlled expansion, selection, icons, and keyboard navigation.
- `tabs` keeps each tab's component state while another tab is visible.
- Application structure through `app_shell`, titled `card` containers, and `status_bar`.
- Selectable `card` containers with keyboard and pointer activation.
- Resizable sidebars and panes through `sidebar` and `splitter`.
- `scroll` measures its content or accepts an explicit extent.
- Popup and context menus through `menu` and `context_menu`, plus an in-window `menu_bar`.
- `command_menu` filters and ranks commands as you type, with nested submenus and keyboard navigation.
- Modal dialogs and side panels through `modal`, with focus trapping and configurable dismissal.
- Hover hints through `tooltip` and keyboard hints through `hint_overlay`.
- Transient messages through `toast` and `toast_queue`.
- Status and progress indicators through `chip`, `dot`, `spinner`, `progress`, and `gauge`.
- Metric tiles through `stat`, with optional change indicators, captions, and trend lines.
- Bar, line, and candlestick plots through `chart`, with negative values, zoom, pan, and pointer readouts.
- Compact trend graphs through `sparkline`, accepting static or reactive values.
- Color grids through `heatmap`.
- Key/value displays through `kv_grid`, with reactive labels and values.
- `markdown` renders headings, links, code, tables, and read-only task lists. `markdown_edit` provides editing with a preview.
- `html` renders a subset of HTML.
- `mermaid` renders diagrams from static or reactive source text.
- `diff` renders highlighted unified or split views, with virtualized rows and hunk or line selection.
- `code_edit` provides a code editor with syntax highlighting.
- Images through `image`, with PNG and JPEG decoding through `ImageSource::decode`.
- Vector paths through `path` and custom drawing through `widgets::canvas`, using the same drawing commands on both hosts.
- Drag and drop through `drag_source`, `drop_target`, and `drag_ghost`, plus desktop file-drop handlers.

#### Styling and themes

- Typed styling through `Paint` and the `Widget` state methods, including typography, borders, shadows, and paint for hover, focus, press, and disabled states.
- Light, dark, and console themes through `Theme`, with shared metrics, semantic colors through `Token`, and color families through `Tone`.
- Theme switching and TOML theme files through `Theme: FromStr`, `Theme::apply`, `watch_theme`, and `AppConfig::theme_file`, including reload when the file changes.
- Right-to-left layout, locale-aware numbers and dates, and translated widget labels through theme direction, `style::Locale`, and `style::Strings`.

#### Text

- Static and reactive text through `text`, accepting strings, signals, or closures, with optional single-line rendering.
- Text shaping, bidirectional text, and IME composition for text inputs.
- Selectable text through `Widget::selectable`, with clipboard-result callbacks through `Widget::selection_copy`.
- Syntax highlighting through `highlight` for Rust, shell, TOML, JSON, Python, JavaScript, TypeScript, Go, C/C++, YAML, CSS, HTML, SQL, Markdown, and diffs.

#### Animation

- Spring and tween style transitions, chosen by `Easing`.
- Style transitions and animated appearance or disappearance through `transition` and `presence`.
- Continuous motion through `animation::clock`, a reactive elapsed-time clock that advances on painted frames.
- Reduced-motion preferences through `animation::reduced_motion`, with OS defaults and an `AppConfig::reduced_motion` override.

#### Window host

- Desktop windows on Linux, macOS, and Windows, and a browser canvas on WebAssembly through `AppConfig::canvas`.
- GPU rendering through wgpu and software rendering through the CPU backend. `SIGIL_BACKEND` selects automatic fallback, GPU, or CPU rendering.
- Multiple windows through `Window::open`, sharing one event loop.
- Exit cleanup through `AppConfig::exit`, including window closure and operating-system quit or shutdown.

#### Terminal host

- The same widget tree on a character grid through `AppConfig::terminal`, with a configurable frame interval.
- Terminal capability detection through `Terminal::caps`, including color depth, background brightness, keyboard protocols, and glyph fallbacks.
- Images through Kitty, iTerm2, and Sixel protocols, with a glyph fallback.
- Keyboard input, bracketed and multi-line paste, mouse clicks and drags, and OSC 52 clipboard support.
- Inline mode through `AppConfig::inline`, preserving scrollback. `Terminal::print_above` writes output above the live region.
- External event integration through `AppConfig::terminal_hooks`, and temporary suspension through `Terminal::suspend`.

#### Accessibility

- AccessKit screen-reader support with roles, names, focus, values, live regions, and accessible document links. `Host::accessibility_tree` exposes the tree to embedded applications.
- Screen-reader announcements through `sigil::node::announce(text, Live::Polite)` and `announce(text, Live::Assertive)`.
- Terminal screen-reader support with focus tracking, announcements, and an accessibility transcript.

#### Platform features

- Async file pickers and alerts through `app::dialog`. The `dialog` feature provides in-app dialogs on both hosts; `native-dialog` adds operating-system dialogs to the window host.
- Native menus through `app::menu`, with shortcut accelerators and menu events.
- System tray icons through `app::tray`, with updates through `Tray::update`.
- Desktop notifications through `app::notify`.
- Global hotkeys through `app::hotkey`.
- Now-playing information and media-key events through `app::media`.
- Audio playback through `app::Player`, with queued tracks, volume control, and reactive playback state.
- Pseudo-terminal child processes through the optional `pty` feature.
- Text and image clipboard access through `clipboard::copy`, `clipboard::paste`, and optional image support.
- Files and URLs opened with the operating-system handler through `app::open_url`.

#### Tooling

- An optional inspector through the `inspect` feature and `SIGIL_INSPECT`, with accessibility queries, screenshots, input control, and frame profiles.
- Headless interaction testing through `sigil_kittest::Harness`, locating controls by accessible label and running async work as frames advance.
- Text and PNG snapshot comparisons through `Harness::snapshot`.
- macOS app bundles and Windows app directories through `sigil_sign::macos_app` and `sigil_sign::windows_dir`. `sigil_sign::CodeSign` signs macOS bundles and standalone executables; bundles can include a provisioning profile.
- Update-package signatures through `sign_file` and `verify_file`, binding the file name and expected version. `sigil-pack` provides packaging, signing, verification, and key generation.
- Optional code hot reload, including `Host::mount_hot` for embedded applications, preserving state created outside the root component and restoring focus.

#### Fonts and icons

- Bundled Fira Sans, Cascadia Mono, and a Noto Color Emoji subset in `sigil-fonts` under SIL Open Font License 1.1. The DejaVu Sans symbol subset uses the Bitstream Vera license; the crate license is OFL-1.1 AND Bitstream-Vera.
- Lucide icons through `Icon` and `icon`, tinted by the active text color.
