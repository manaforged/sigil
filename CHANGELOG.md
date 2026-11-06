# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html). The `sigil-ui`, `sigil-fonts`, `sigil-sign`, and `sigil-kittest` crates share one version.

## [Unreleased]

### Added

- `View` and `view!`: a widget tree in a `.sigil` file that the running app reloads on save, with no rebuild. A mistake keeps the previous screen and shows the line and column.
- `Space`, `TextRole`, and `Layer`: named steps for spacing, type, and depth. `gap`, `padding`, and `margin` setters no longer take a bare number, and `font_size` takes a reason.
- A `Harness` runs the design rules when it is dropped. See `FlawKind`, `Harness::assert_no_flaws`, `Harness::unaudited`, and `Widget::allow_flaw`.
- Keyframe animations: `Paint::animate(Keyframes)`, with `Keyframes` stops, per-stop easing, iterations, `reverse` and `alternate`, and the Tailwind `spin`, `ping`, `pulse` and `bounce` presets. `Property` names the animated value.
- CSS-style transitions on `Paint`: `transition`, `transition_all`, `transition_colors`, `transition_opacity`, `transition_transform`, `duration`, `ease` and `delay`.
- Vector paths draw on the GPU through `vello_gpu` and on the CPU through `vello_cpu` instead of a tiny-skia bitmap. `PathNode` gains `even_odd`, `cap(StrokeCap)`, `join(StrokeJoin)` and `dash`; `scene::StrokeCap` and `scene::StrokeJoin` are new.
- Vega-Lite fences in a window render through the official vega 6.4.0 and vega-lite 6.5.0 running in an embedded QuickJS, with a 2 s deadline, a 256 MB heap cap and no network; the native mapping is the fallback and the terminal path. The figure is named by the spec title and described by its axis titles.
- `ChartBuilder::horizontal()` lays bars along rows.
- `Easing::CubicBezier`, `Easing::cubic_bezier` and the `Easing::IN`, `OUT` and `IN_OUT` curves.
- Radial and conic background gradients: `Paint::bg_radial(Radial)` with `RadialExtent`, `Paint::bg_conic(Conic)`, `Paint::bg_linear(GradientAxis)`, the `from`, `via` and `to` stops with positions, and repeating gradients. `scene::Gradient` and `scene::GradientKind` carry linear, radial and conic paints.
- `Paint::bg_clip_text` paints the background gradient through the glyphs; `Paint::bg_image` tiles an image behind the content.
- `Paint::border_dashed`, `border_dotted` and `border_solid`; dashes follow rounded corners.
- `Paint::clip_path(ClipShape)` clips a node, its background and its children to an inset, circle, ellipse or polygon, and pointer hits outside the shape miss.
- Masks on `Paint`: `mask_image(MaskImage)` with linear, radial, conic and image masks, `mask_stops`, `mask_luminance`, `mask_size(MaskSize)`, `mask_position` and `mask_repeat`, and the Tailwind `mask-*` classes for them.
- `Paint::mix_blend(BlendMode)` and the `mix-blend-*` classes.
- Filters on `Paint`: `blur`, `brightness`, `contrast`, `grayscale`, `hue_rotate`, `invert`, `saturate`, `sepia`, `drop_shadow` and `text_shadow`, with the Tailwind `blur-*`, `brightness-*`, `contrast-*`, `grayscale`, `hue-rotate-*`, `invert`, `saturate-*`, `sepia`, `drop-shadow-*` and `text-shadow-*` classes.
- 3D transforms on `Paint`: `perspective`, `perspective_origin`, `rotate_x`, `rotate_y`, `rotate_z`, `translate_z`, `backface_hidden` and `backface_visible`, with `Property::RotateX`, `RotateY` and `TranslateZ` for keyframes.
- Keyframes and transitions animate filters, drop and text shadows, gradient stop colors and positions, mask stops and the 3D turns, depth and perspective. `Property` gains `RotateZ`, `Perspective`, `Blur`, `Brightness`, `Contrast`, `Grayscale`, `HueRotate`, `Invert`, `Saturate`, `Sepia`, `DropShadow`, `TextShadow`, `GradientFrom`, `GradientVia`, `GradientTo`, their `*Position` forms, `MaskFrom` and `MaskTo`. Filter lists, gradients and shadows interpolate as CSS does and jump when they do not match.
- Class strings parse `bg-linear-to-*`, `bg-radial`, `bg-conic-*`, `from-*`, `via-*`, `to-*`, `bg-clip-text`, `border-solid`, `border-dashed` and `border-dotted`.

### Changed

- Vega-Lite charts and SVG icons (`icon(Icon::*)`, `icon_svg`) lower to scene commands like Mermaid instead of a CPU bitmap, so no vector widget rasters on the CPU in a window build. `icon_svg` now keeps explicit colors in the SVG; `currentColor` and a root with no `fill` take the tone's color.
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

### Migration

| Before | After |
| --- | --- |
| `icons::bake`, `icons::BakedIcon`, `icons::QuadCurve`, `icons::cubic_to_quadratics`, `renderer::scenes::BUILTIN_ICON_ATLAS_BASE_ID` (public only under `kittest`) | removed; `icon(Icon::*)` and `icon_svg` draw paths |
| `SceneCommand::PushClip(Rect)` | `SceneCommand::PushClip(Clip)`; pass `rect.into()` |
| `const DrawGradient::new(rect, LinearGradient)` | `DrawGradient::new(rect, impl Into<Gradient>)`, not `const`; a `LinearGradient` converts with `From` |
| `LayerScope { clip, clip_radii, effect, opacity }` | add `blend: BlendMode::Normal`, or use `..` with a base scope |
| exhaustive `match` on `LayerEffect` | add arms for `Projected`, `Mask` and `Filter` |
| `LayerEffect::LayerBlur { sigma }` | `LayerEffect::Filter(Arc::new([Filter::Blur(sigma)]))` |
| exhaustive `match` on `BlendMode` or `Filter` | add a `_` arm; both are `#[non_exhaustive]` |

## [0.2.0] - 2026-11-02

First public release.

### Migration

| Before | After |
| --- | --- |
| `Host::misalignments`, `Host::emit_since`, `Cx::take_delta`, `LayoutStyle::data` | `sigil::testing::misalignments`, `emit_since`, `take_delta`, `style_data` (feature `kittest`) |
| `LayoutStyle::with_layout`, `events::EventRouter`, `style::StyleBinding`, `style::ReactiveStateSlot`, `style::ReactiveStyleAtoms`, `style::StyledTextEngine` | public only under feature `kittest` |
| `GlyphRun::positioned_glyphs` | `GlyphRun::glyph_count` for the count; the glyphs are crate-private |
| `Node::tag`, `TextStyle::layered_over_binding`, `FrameInfo::delta` | crate-private |
| `sigil::kernel::*`, `sigil::layout::*` | prelude (`text`, `HostMode`, `TextBuilder`, `ImageBuilder`, `ObjectFit`, `TextCursor`, `Dimension`, `Position`, `Overflow`, `TrackSize`) or crate root (`Cx`, `NodeId`, `Node`, `NodeType`, `NodeExtensions`, `AccessibleName`, `TopLayer`, `Surface`, `ImageCrop`, `ImageRaster`, `Live`, `FocusScopeKind`, `announce`) |
| `sigil::animation_runtime::{Animator, AnimatorStatus, Playback, clock, reduced_motion}` | `sigil::animation::*` |
| `sigil::text::{HitPosition, TextAffinity}`, `TextMeasure`, `CaretGeometry`, `app::DefaultTextMeasure`, `profile::{note_input, FRAME_BUDGET_NS}`, `widgets::MIN_COL_WIDTH`, `DrawRect::border_radius`, `DrawRect::border` | public only under `kittest`, `#[doc(hidden)]`; the crate-root paths are `sigil::TextMeasure`, `sigil::CaretGeometry`, `sigil::HitPosition`, and `sigil::TextAffinity` |
| `sigil::text::{WebFont, WebFontScript, add_fallback_font, load_font_url, set_web_fallbacks, web_fallbacks}` | `sigil::app::*` |
| `sigil::a11y::snapshot(&host.accessibility_tree())` | `host.accessibility_text()` |
| `sigil::a11y`, `sigil::kernel`, `sigil::layout`, `sigil::animation_runtime`, `sigil::text`, other items in them | public only under `kittest`, `#[doc(hidden)]` |
| `layout::Rect<T>` (edges), `layout::Size<T>` (per axis) | `Edges<T>`, `Axes<T>`, under `kittest`. `geometry::Rect` stays the box |
| `style::DisplayKind`, `FlexWrapKind`, `OverflowKind`, `AlignKind`, `JustifyKind` | `Display`, `FlexWrap`, `Overflow`, `AlignItems`, `JustifyContent`. `JustifyKind::Between`, `Around`, `Evenly` are `JustifyContent::SpaceBetween`, `SpaceAround`, `SpaceEvenly`. `AlignKind::Between` and `Around` are gone; the class grammar never produced them |
| `AlignItems::FlexStart`, `AlignItems::FlexEnd` | `AlignItems::Start`, `AlignItems::End` |
| `ColorOrToken`, `ColorKey`, `ColorAtom::ThemeRef(ColorKey)` | `ColorAtom::{Rgba8, Token, Custom}`. `ColorOrToken::Token(ColorKey::Custom(name))` is `ColorAtom::Custom(name)` |
| `StyleAtom::BackgroundThemeRef`, `TextThemeRef`, `BorderThemeRef` | `StyleAtom::BackgroundColor`, `TextColor`, `BorderColor` |
| `Theme::resolve_color(ColorKey)`, `TokenLayer::color(ColorKey)` | take `ColorAtom` |
| `.animate(NumericAtomKind::Height, value)` | `.animate_height(value)`, with one `animate_*` setter per property. `NumericAtomKind` and `ColorAtomKind` are private |
| `Rgba8: FromStr` accepted `#` plus 3 or 6 digits | also accepts 8 digits and a `0x` or missing prefix. Theme files use it, so they also accept 3 digits |
| `SceneRectStyle`, `SceneGradientStyle`, `ScenePatternStyle`, `SceneSpriteStyle`, `SceneDecorationStyle` | setters on `DrawRect`, `DrawGradient`, `DrawPattern`, `DrawSprite`, `DrawDecoration` |
| `scene.draw_rect(rect, SceneRectStyle::solid(color))` | `scene.draw_rect(DrawRect::new(rect, color))`. `draw_gradient`, `draw_pattern`, `draw_decoration`, and `draw_sprite` take the draw command too |
| `DrawGlyphRun`, `draw.run.text` | `SceneCommand::GlyphRun(GlyphRun)`, `draw.text` |
| `Axis::X`, `Axis::Y` | `Orientation::Horizontal`, `Orientation::Vertical` |
| `ToastLevel`, `ToastEntry::level`, `ToastLevel::Warning` | `Tone`, `ToastQueue::push(tone, message)`, `Tone::Warn` |
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
| `widgets::NumericKind`, `widgets::FuzzyMatch`, `widgets::HintEntry`, `reactive::Subscription` | no path. `number_field` takes the built-in number types, `fuzzy_score` and `subscribe` return the value without a name, and `hint_overlay` takes `&[(&str, &str)]` |
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
| `terminal::present` | `Harness::present` |
| `terminal::step(host, events, cols, rows)` | `host.grid(events, cols, rows)` |
| `terminal::render(out, prev, next, depth)` | `Host::ansi(out, prev, next, depth)` |
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
| `parse_theme`, `parse_hex` | `Theme: FromStr`, `Rgba8: FromStr`. 3 digits and surrounding whitespace are newly accepted, and `+12345` is newly rejected |
| `pub use wgpu` | removed |
| functions that returned `std::io::Result` | return `sigil::Error` |
| `Dimension::px` | `IntoPx::into_px`. `Dimension` does not implement `Eq` or `Hash` |
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
| `PasteEvent`; `on_paste(Fn(&mut PasteEvent))` | `on_paste(Fn(&str) -> bool)`; return true where the handler called `stop_propagation`. `Event::paste(text)` builds the event |
| `terminal::key_down(key)`, `terminal::paste(text)` | `Event::key_down(key)`, `Event::paste(text)` |
| `terminal::pointer(phase, col, row)`, `terminal::click(col, row)`, `terminal::wheel(col, row, lines)` | `Event::pointer_down`, `pointer_up`, `pointer_drag`, `click`, `wheel` at the layout point of the cell (`Host::CELL` points per cell). `Event::click` also sends the move before the press |
| `sigil_kittest::input::{pointer_move, pointer_drag, pointer_down, pointer_up, click}(x, y)` | `Event::pointer_move`, `pointer_drag`, `pointer_down`, `pointer_up`, `click` with `Point::new(x, y)` |
| `input::key_event(key, physical, phase)`, `key_down(key, physical)`, `chord(key, physical, modifiers)` | `Event::key(key, phase).physical(physical)`, `Event::key_down(key).physical(physical)`, adding `.modifiers(modifiers)` for a chord |
| `input::press`, `type_char(text, physical)`, `type_text`, `tab`, `scroll_lines(x, y, dy)`, `window_blur`, `ime_event(event)` | `Event::press`, `Event::typed(text).physical(physical)`, `Event::type_text`, `Event::tab`, `Event::wheel(Point::new(x, y), dy)`, `Event::blur`, `Event::Ime(event)` |
| `app::TextTuning`, `set_text_tuning`, `text_tuning` | `AppConfig::text_enhance`, `text_snap`, `text_raster`, `text_dilation`, `text_sharpen`, `text_gamma` |
| `app::DefaultFonts`, `set_default_fonts` | `AppConfig::font_sans`, `font_sans_weight`, `font_mono`, `font_icon`, `font_display`, `font_fallback` |
| `animation_runtime::MotionPrefs::reduced()`, `MotionPrefs::set` | `animation_runtime::reduced_motion()`, `AppConfig::reduced_motion` |
| `app::HostCaps`, `Host::set_caps(HostCaps { hyperlinks })`, `Caps::host()` | `Host::set_hyperlinks(hyperlinks)` |
| `app::WindowOpts` | `AppConfig::overlay`, `frameless`, `anchor`, `user_zoom`, `min_size`, `resizable` |
| `reactive::effect_named` | `reactive::effect`. Named effects are crate-private |
| `tasks::on_message(rx, f)`, `on_message_or_close(rx, f, on_close)` | `tasks::on_message(rx, f, on_close)`; pass `\|\| {}` when nothing runs on close |
| `task_with`, `Task::run_with`, `Task<T, E, A>` | `task` and `Task<T, E>`; capture the argument in a signal the closure reads |
| `reactive::cancel_interval(id)` | `IntervalId::cancel` |
| `kernel::announce(text)`, `announce_assertive(text)` | `kernel::announce(text, Live::Polite)`, `announce(text, Live::Assertive)` |
| `scene::emit_scene(cx, root)`, `emit_scene_clipped(cx, root, clip)` | `scene::emit_scene(cx, root, clip)`; pass `None` for no clip |
| `list_row(selected, title, detail)`, `list_row_with(selected, title, slots)`, `ListRowSlots::row` | `list_row(selected, title, slots)`; `detail` is `ListRowSlots::status` |
| `image_rgba(w, h, rgba)`, `image_bytes(encoded)` | `ImageSource::from_rgba8(w, h, rgba).map(image)`, `ImageSource::decode(encoded).map(image)`; add `.layout(LayoutStyle::block())` for the old layout |
| `app::open_window(config, view)` | `WindowId::open(config, view)` |
| `Host::new()`, `Host::cells()`, `Host::default()` | `Host::new(HostMode::Pixel)`, `Host::new(HostMode::Cell)` |
| `tasks::TaskWorkers`, `AsyncRuntime::with_workers`, `AppConfig::task_workers(TaskWorkers)` | `AppConfig::task_workers(NonZeroUsize)` |
| Feature `render` | Feature `scene` |
| `AppConfig::title()`, `window_size()`, `exits_on_esc()`, `on_panic()` | removed. The app sets these values and keeps them |
| `Host::apply_focus_requests`, `Host::sync_hover` | `Host::frame` runs them |
| `FrameInputHook`, `FrameLayoutHook`: `FnOnce(&mut Host, &mut EventRouter)`; `FramePaintHook`: `FnOnce(&mut Host, &mut EventRouter, &FrameInfo)` | `FnOnce(&mut Host)`; `FnOnce(&mut Host, &FrameInfo)` |
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
| `style::StateVariants` (`rest`, `hover`, `focus`, `pressed`, `disabled_style`) | the same trait, implemented only for a builder that has state slots |
| `NodeBuilder::tone(Tone)`, `TextBuilder::tone(Tone)` (from `NodeStyleExt`) | `Widget::tone_scope(Tone)`; `tone` stays the widget tone on button, chart, chip, checkbox, icon, progress, radio group, sparkline, and stat |
| `TooltipBuilder::shortcut(Option<impl Into<String>>)` | `TooltipBuilder::hint(Option<impl Into<String>>)` |
| `CodeEditBuilder::key(f)` | `CodeEditBuilder::key_handler(f)` |
| `ChipBuilder::layout(LayoutStyle) -> NodeBuilder` | `Widget::layout(LayoutStyle) -> ChipBuilder` |
| `CanvasBuilder::layout`, `PaneBuilder::grow`, `TextFieldBuilder::autofocus`, `TextFieldBuilder::focus_when`, `TextEditBuilder::autofocus`, `TextEditBuilder::focus_when` | the `Widget` methods of the same name, with the same effect |
| `VerticalDivider`, `divider_vertical() -> VerticalDivider` | `divider_vertical() -> NodeBuilder` |
| `PathNode::layout(LayoutStyle)`, which replaced the `fit` box | `Widget::layout`, which merges into it |
| `widgets::layout_ext::LayoutOverrides` | `Widget::layout`, `width`, `height`, and `grow` |
| A second `.layout(..)` on a widget builder replaced the first | Each `.layout(..)` merges, in call order, as on `NodeBuilder` |
| `const fn color_field` | `fn color_field` |
| `const fn divider_vertical` | `fn divider_vertical` |
| `widgets::layout_ext` | private; its methods are `Widget` |
| `TextFieldBuilder::autofocus`, `TextFieldBuilder::focus_when`, `TextEditBuilder::autofocus`, and `TextEditBuilder::focus_when` as `const fn` | the same methods, no longer `const` |
| `Placement` without `mark_col`, `mark_row`, `span_cols`, and `span_rows` | those fields are part of `Placement` |
| `ColorFieldBuilder` and `MarkdownEditBuilder` are `UnwindSafe` | they are not; their callback fields are trait objects |
| `TextBuilder::label`, `description`, and `keyboard_shortcut` as `const fn` | the same methods, no longer `const` |
| `text(...).map_node` and `image(...).map_node` run at the call | they run when the node is built, after `layout`, `width`, `height`, and `grow` |
| a width or height hint ignores a size set through `layout` | it follows a size set through `layout` as well as through `width` and `height` |
| `*Builder` types a function returns (`CardBuilder`, `CheckboxBuilder`, `ChipBuilder`, `SwitchBuilder`, `StatusView`, and others) as nameable paths | the function returns them and their methods work; write `-> impl IntoNode` where a signature named one. `ButtonBuilder`, `ChartBuilder`, `DiffBuilder`, `IconBuilder`, `ImageBuilder`, `ModalBuilder`, `NodeBuilder`, `SparklineBuilder`, `TextBuilder`, `TextEditBuilder`, `TextFieldBuilder`, and `VirtualListBuilder` stay |
| bound traits `IntoPx`, `IntoIconSource`, `IntoScrollExtent`, `IntoList`, `DiffSource`, `FieldControl`, `MessageReceiver` as nameable paths | the bounds still apply; pass the same values |
| `animation::{Animator, Spring, SpringPreset, Tween}` | crate-private; `Animator` and `SpringPreset` stay public only under `kittest` |
| `animation::CubicBezier` | removed with `Easing::Bezier` |
| `animation::{Keyframe, Keyframes, AnimationDirection, AnimationFill, Playback, AnimatorStatus}`, `terminal::CellGrid`, `style::ThemeBuilder`, `widgets::FocusCaret`, `TextFieldBuilder::focus_caret`, `Widget::presence_spring`, `Widget::ime_caret` | removed; no caller used them |
| `Paint::{background_gradient, transform, transition, duration}`, `LayoutStyle::{align_content, box_sizing, flex_direction}`, `SceneBuilder::{draw_gradient, draw_pattern, draw_sprite, push_layer, draw_decoration, draw_image, draw_shadow, with_composite_layer}`, `DrawRect::border_radius`, `Theme::{with_locale, with_strings, with_*_scale}`, `Date::{parse_in, render_in}`, `app::{set_ambiguous, set_cell_width}`, `style::{render_style_for, text_style_for}` | public only under `kittest` |
| `style::scale`, `style::{Locale, Strings, StyleAtom, StateStyle, StyleState, StyleVariant, StyleEngine, RenderStyle, Transform2D, TransitionId, MillisU16, BgGradient}` and the other style value types | public only under `kittest` or crate-private; `Token::scale` and `Tone::scale_role` still work without naming `ScaleRole` |
| `scene::{DrawGradient, DrawImage, DrawPattern, DrawSprite, DrawDecoration, LayerEffect, LayerScope, LinearGradient, GradientStop, CornerRadii, StrokeAlign, StrokeStyle}` and the other command payloads | public only under `kittest` or crate-private; `SceneCommand` fields keep these types |
| `AccessibleName`, `FocusScopeKind`, `NodeExtensions`, `ImageCrop`, `icons::{IconBaker, BakedIcon}` (now `icons::bake`), `events::FileDropEvent`, `reactive::DrawPhase` | public only under `kittest`; `.label("text")` and `.file_drop(\|event\| ...)` work without the names |
| `app::{FrameOutput, FrameLayoutHook, Ambiguous, CellWidth}`, `terminal::{AfterFrame, EventPump, ClipRoute, Erase, Glyphs, Notices, Placement}`, `profile::{Attribution, PhaseTime, CounterValue}`, `tasks::TaskHandle`, `widgets::{ButtonGlyph, LineId, LineKind, PathCmd, PathColor, PathFill, TreeRow, IconSource}`, `events::PointerTilt` | no public path; functions return them and fields keep them |
| `style::palette::Palette` | crate-private. Use `Theme` |
| `geometry::Scale`, `Cx::scale() -> Scale`, `Cx::set_scale(Scale)` | `Cx::scale() -> f32`, `Cx::set_scale(f32)`. `Scale` is public only under `kittest` |
| `widgets::ColumnLayout`, `TableState::column_layout` | `TableState::hide_column`, `show_column`, `visible_columns`, `column_width`; the layout is crate-private |
| `Shadow::kind`, `DrawRect::stroke` fields | public only under `kittest` |
| `reactive::{Runtime, next_interval_deadline, debug}`, `style::unknown_class_warning_count`, `scene::{paint_backdrop, shape_glyph_run, Stroke}`, `terminal::{map_key, map_mouse, cell_point}`, `widgets::interaction`, `A11yProps`, `Cx::new_node`, `clipboard::NoopClipboard`, `DrawRect::stroke`, `Shadow::with_kind`, `TextBuilder::runs` | public only under `kittest` |
| `style::{variant, Variant, is_typography_atom}`, `Theme::from_palette`, `terminal::{install_clipboard, blocks_window, png_window, sextants_window, detect_mux}`, `widgets::{listbox, ListModel, language_for_path, text_runs, ToastEntry}`, `app::{cell_text_lines, ellipsize_cells, ellipsize_cells_start, fit_cells}`, `tasks::panic_message`, `style::font_family_hint` | no public path; `Caps` reports the multiplexer and `ToastQueue` owns its entries |
| `widgets::to_hex(color)` | `Rgba8::to_hex(self)` |
| `Node::a11y.text_cursor` | `Node::text_cursor()`; the `a11y` field is public only under `kittest` |
| `Widget::focus_ring(FocusRing)`, `style::FocusRing`, `FocusRing::{new, static_visible, color, theme_color, width, offset, opacity}` | `Widget::focus_ring(Signal<bool>)`, `focus_ring_color`, `focus_ring_width`, `focus_ring_offset`, `focus_ring_opacity`. A fixed ring is `focus_ring(signal(true))`. `style::FocusRing` is public only under `kittest` |
| `match_status`, `StatusView` | `show` on `Task::status` |
| `events::request_reveal`, `app::{window_focused, window_occluded}`, `widgets::TreeKey` | removed. No caller used the first two. `WindowId::focused` and `WindowId::occluded` stay. `TreeKey` is `Clone + Eq + Hash + 'static` |
| `app::{current_window, use_system_fonts, add_fallback_font}` | crate-private; `AppConfig::font_fallback` adds a fallback face |
| `tasks::installed`, `app::web_fallbacks`, `scene::Transform`, `kernel::NodeType`, `Node::kind`, `terminal::{glyphs, base64, png, blocks, sextants, detect}`, `profile::summary` | public only under `kittest`. `Caps` reports the glyphs and protocol. A `Transform` is `[[f32; 3]; 2]` |
| `icons::builtin`, `events::PRIMARY`, `style::{DISABLED_OPACITY, ICON_SIZE, OVERLAY_SHADOW}`, `widgets::{DRAG_THRESHOLD_PX, DROP_TARGET_CLASS, MAX_VISIBLE, TOAST_TTL, ChartMark}`, `profile::{count, gauge, enabled, Category}`, `Phase::category` | crate-private. `Shortcut::primary`, `Metrics::icon_size`, `count_add`, and `ChartBuilder::{bar, line, candles}` do those jobs |
| `profile::{Frame, Span}`, `reactive::EventChannel` as nameable paths | the functions that return them still work; `EventChannel::{new, emit}` are public only under `kittest` |
| `Menu::install(self)`, `Tray::install(self)` | both return `AppResult<()>`. `Menu::install` fails on a platform with no native menu bar, and `Tray::install` fails on an invalid icon image. A later native failure is logged. A shortcut key with no native mapping keeps its item without the accelerator and logs a warning. `F13` to `F24` and the other keys `muda` supports now map |
| `Golden` paths from the compile-time `CARGO_MANIFEST_DIR` of the test crate | `Golden::new` reads `CARGO_MANIFEST_DIR` when the test runs. `cargo test` sets it. A test binary run directly must set it, or the path is relative to the current directory |
| `FocusScopeKind::Inert` under `kittest` | removed; nothing used it |
| receive-only `CommandMenuBuilder`, `DateFieldBuilder`, `DotBuilder`, `DropdownBuilder`, `FieldBuilder`, `GaugeBuilder`, `HintOverlayBuilder`, `InteractiveBuilder`, `MenuBarBuilder`, `MenuBuilder`, `NumberFieldBuilder`, `PanelBuilder`, `ProgressBuilder`, `RadioGroupBuilder`, `ScrollBuilder`, `SegmentedBuilder`, `SidebarBuilder`, `SliderBuilder`, `SplitterBuilder`, `StatBuilder`, `StateStyleBuilder`, `SubmenuBuilder`, `TableBuilder`, `TabsBuilder`, `ToggleBuilder`, `TreeBuilder` as nameable paths | the function returns them and their methods work; write `-> impl IntoNode` |
| `CellBorder`, `CellRect`, `ColorScale`, `ComponentId`, `DateOrder`, `DecorationKind`, `FontFamilyHint`, `GradientAxis`, `GradientStopKey`, `PaintFill`, `PatternKind`, `ShadowKind`, `SpriteKind`, `SpriteSource`, `StateSlot`, `TextDecoration`, `TextFace`, `TextFlow`, `TextRun`, `TokenTable`, `ToneContext`, `VariantTokens`, `AnimatorOverlay`, `Asymmetric`, `AttachedAnimator`, `AttachedAnimators`, `AttachedColorAnimator`, `BoundsReactive`, `DecorationGeometry`, `DeltaKind`, `EmitDiff`, `InteractionState`, `LayoutEngine`, `LayoutResult`, `Misalignment`, `MisalignmentKind`, `NodeLabel`, `NodeTag`, `OwnedTextEngine`, `Paragraph`, `ParagraphMetrics`, `ReactivePosition`, `StyleOverlay`, `StyleReactiveEffect`, `TextEngine`, `TextLineHeight`, `TextRuns`, `Tolerance` | crate-private or public only under `kittest`; no caller's non-test code named them |

### Added

- One error type for the crate: `sigil::Error` with `sigil::ErrorKind` replaces 13 module error types. `docs/guide/errors.md` maps each old type to its kind.
- The host traits, the renderer, and the window host are crate-private. The `kittest` feature exposes them to the harness. Text measurement types are `sigil::text`.

#### Core

- A retained widget tree built from component functions whose results implement `IntoNode`. `sigil::run` runs the application on the selected host.
- `Host` embeds the same tree in another engine's event loop, with event routing, layout, drawing, and host-rendered `surface` regions.
- Reactive state through `signal`, `derived`, and `effect`. `batch` groups writes, and `interval` runs periodic callbacks.
- Conditional views through `show` and `switch`, with optional state preservation for hidden views.
- Keyed collections through `for_each`, preserving each item's node and state while its key remains.
- Async work through `task`, `task_with`, and `spawn`, with reactive status and cancellation for tasks. `match_status` renders loading, result, and error views.
- `tasks::blocking` runs blocking work inside async tasks. `signal_writer` and `tasks::on_message` deliver results from other threads to the UI thread.
- Flexbox and grid layout through `LayoutStyle`, `row`, and `column`, with pixel sizes and text units `rows`, `cols`, and `em`.
- Keyboard, pointer, scroll, paste, and gesture handlers through `Widget`. `Shortcut` parses and displays key combinations, with `Shortcut::primary` selecting Command or Control for the platform.

#### Widgets

- Push buttons through `button` and `icon_button`, with independent tone and fill choices, disabled states, and activation callbacks.
- Boolean controls through `checkbox` and `toggle`.
- Single-choice controls through `radio_group` and `segmented`.
- Numeric sliders through `slider`, with step sizes and horizontal or vertical orientation.
- Single-line input through `text_field`, with password masking, leading and trailing content, and a clear button.
- Multi-line input through `text_edit`, with selection, reactive focus, and configurable submission keys.
- Numeric input through `number_field`, with range limits and locale-aware parsing.
- Date input through `date_field`, using the theme's date format.
- Color input through `color_field`, with a swatch and hexadecimal entry.
- Form labels and validation through `field`, with required markers, error messages, and accessible descriptions.
- Choice fields through `dropdown` and searchable `combobox`.
- Sortable, filterable tables through `table` and `TableState`, with row selection and configurable columns.
- `virtual_list` builds only visible rows, supports measured row heights, and can follow a row or the end of the list. Measured lists with stable keys preserve the visible item when rows are inserted above it.
- Explorer and outline views through `tree` and `TreeNode`, with controlled expansion, selection, icons, and keyboard navigation.
- `tabs` keeps each tab's component state while another tab is visible.
- Application structure through `app_shell`, titled `panel` containers, and `status_bar`.
- Selectable `card` containers with keyboard and pointer activation.
- Resizable sidebars and panes through `sidebar` and `splitter`.
- `scroll` measures its content or accepts an explicit extent.
- Popup and context menus through `menu` and `context_menu`, plus an in-window `menu_bar`.
- `command_menu` filters and ranks commands as you type, with nested submenus and keyboard navigation.
- Modal dialogs through `modal` and side panels through `sheet`, with focus trapping and configurable dismissal.
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
- Images through `image`, with PNG and JPEG decoding through `image_bytes`.
- Vector paths through `path` and custom drawing through `widgets::canvas`, using the same drawing commands on both hosts.
- Drag and drop through `drag_source`, `drop_target`, and `drag_ghost`, plus desktop file-drop handlers.

#### Styling and themes

- Typed styling through `Paint` and the `Widget` state methods, including typography, borders, shadows, and paint for hover, focus, press, and disabled states.
- Light, dark, and console themes through `Theme`, with shared metrics, semantic colors through `Token`, and color families through `Tone`.
- Theme switching and TOML theme files through `Theme: FromStr`, `set_theme`, `watch_theme`, and `AppConfig::theme_file`, including reload when the file changes.
- Right-to-left layout, locale-aware numbers and dates, and translated widget labels through theme direction, `style::Locale`, and `style::Strings`.

#### Text

- Static and reactive text through `text`, accepting strings, signals, or closures, with optional single-line rendering.
- Text shaping, bidirectional text, and IME composition for text inputs.
- Selectable text through `Widget::selectable_text`, with clipboard-result callbacks through `on_selection_copy`.
- Syntax highlighting through `highlight` for Rust, shell, TOML, JSON, Python, JavaScript, TypeScript, Go, C/C++, YAML, CSS, HTML, SQL, Markdown, and diffs.

#### Animation

- Spring and tween style transitions, chosen by `Easing`.
- Style transitions and animated appearance or disappearance through `transition` and `presence`.
- Continuous motion through `animation_runtime::clock`, a reactive elapsed-time clock that advances on painted frames.
- Reduced-motion preferences through `animation_runtime::reduced_motion`, with OS defaults and an `AppConfig::reduced_motion` override.

#### Window host

- Desktop windows on Linux, macOS, and Windows, and a browser canvas on WebAssembly through `AppConfig::canvas`.
- GPU rendering through wgpu and software rendering through the CPU backend. `SIGIL_BACKEND` selects automatic fallback, GPU, or CPU rendering.
- Multiple windows through `app::open_window`, sharing one event loop.
- Exit cleanup through `AppConfig::on_exit`, including window closure and operating-system quit or shutdown.

#### Terminal host

- The same widget tree on a character grid through `AppConfig::terminal`, with a configurable frame interval.
- Terminal capability detection through `terminal::caps`, including color depth, background brightness, keyboard protocols, and glyph fallbacks.
- Images through Kitty, iTerm2, and Sixel protocols, with a glyph fallback.
- Keyboard input, bracketed and multi-line paste, mouse clicks and drags, and OSC 52 clipboard support.
- Inline mode through `AppConfig::inline`, preserving scrollback. `terminal::print_above` writes output above the live region.
- External event integration through `AppConfig::terminal_hooks`, and temporary suspension through `terminal::suspend`.

#### Accessibility

- AccessKit screen-reader support with roles, names, focus, values, live regions, and accessible document links. `Host::accessibility_tree` exposes the tree to embedded applications.
- Screen-reader announcements through `kernel::announce` and `kernel::announce_assertive`.
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
- Files and URLs opened with the operating-system handler through `app::open`.

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
