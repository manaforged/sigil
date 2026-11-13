# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html). The `sigil-ui`, `sigil-fonts`, `sigil-sign`, and `sigil-kittest` crates share one version.

## [Unreleased]

## [0.2.0] - 2026-11-12

First public release.

### Migration

| Before | After |
| --- | --- |
| `Host::misalignments`, `Host::emit_since`, `Cx::take_delta`, `LayoutStyle::data` | `sigil::testing::misalignments`, `emit_since`, `take_delta`, `style_data` (feature `kittest`) |
| `LayoutStyle::with_layout`, `events::EventRouter`, `style::StyleBinding`, `style::ReactiveStateSlot`, `style::ReactiveStyleAtoms` | public only under feature `kittest` |
| `GlyphRun::positioned_glyphs` | `GlyphRun::glyph_count` for the count; the glyphs are crate-private |
| `Node::tag`, `TextStyle::layered_over_binding`, `FrameInfo::delta` | crate-private |
| `sigil::kernel::*`, `sigil::layout::*` | prelude (`text`, `HostMode`, `TextBuilder`, `ImageBuilder`, `ObjectFit`, `TextCursor`, `Dimension`, `Position`, `Overflow`, `TrackSize`) or crate root (`Cx`, `NodeId`, `Node`, `NodeType`, `NodeExtensions`, `AccessibleName`, `TopLayer`, `Surface`, `ImageCrop`, `ImageRaster`, `Live`, `FocusScopeKind`, `announce`) |
| `sigil::animation_runtime::{Animator, AnimatorStatus, Playback, clock, reduced_motion}` | `sigil::animation::*` |
| `sigil::text::{HitPosition, TextAffinity}`, `TextMeasure`, `CaretGeometry`, `app::DefaultTextMeasure`, `profile::{note_input, FRAME_BUDGET_NS}`, `widgets::MIN_COL_WIDTH`, `DrawRect::border_radius`, `DrawRect::border` | public only under `kittest`, `#[doc(hidden)]`, at `sigil::layout::{TextMeasure, CaretGeometry}` and `sigil::text::{HitPosition, TextAffinity}` |
| `sigil::text::{WebFont, WebFontScript, add_fallback_font, load_font_url, set_web_fallbacks, web_fallbacks}` | `sigil::app::*` |
| `sigil::a11y::snapshot(&host.accessibility_tree())` | `host.accessibility_text()` |
| `sigil::a11y`, `sigil::kernel`, `sigil::layout`, `sigil::text`, other items in them | public only under `kittest`, `#[doc(hidden)]` |
| `layout::Rect<T>` (edges), `layout::Size<T>` (per axis) | `Edges<T>`, `Axes<T>`, under `kittest`. `geometry::Rect` stays the box |
| `style::DisplayKind`, `FlexWrapKind`, `OverflowKind`, `AlignKind`, `JustifyKind` | `Display`, `FlexWrap`, `Overflow`, `AlignItems`, `JustifyContent`. `JustifyKind::Between`, `Around`, `Evenly` are `JustifyContent::SpaceBetween`, `SpaceAround`, `SpaceEvenly`. `AlignKind::Between` and `Around` are gone; the class grammar never produced them |
| `AlignItems::FlexStart`, `AlignItems::FlexEnd` | `AlignItems::Start`, `AlignItems::End` |
| `ColorOrToken`, `ColorKey`, `ColorAtom::ThemeRef(ColorKey)` | `ColorAtom::{Color, Token, Custom}`. `ColorOrToken::Token(ColorKey::Custom(name))` is `ColorAtom::Custom(name)` |
| `StyleAtom::BackgroundThemeRef`, `TextThemeRef`, `BorderThemeRef` | `StyleAtom::BackgroundColor`, `TextColor`, `BorderColor` |
| `Theme::resolve_color(ColorKey)`, `TokenLayer::color(ColorKey)` | take `ColorAtom` |
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
| `parse_theme`, `parse_hex` | `Theme: FromStr`, `Color: FromStr`. 3 digits and surrounding whitespace are newly accepted, and `+12345` is newly rejected |
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
| `PasteEvent`; `on_paste(Fn(&mut PasteEvent))` | `paste(Fn(&str) -> bool)`; return true where the handler called `stop_propagation`. `Event::paste(text)` builds the event |
| `terminal::key_down(key)`, `terminal::paste(text)` | `Event::key_down(key)`, `Event::paste(text)` |
| `terminal::pointer(phase, col, row)`, `terminal::click(col, row)`, `terminal::wheel(col, row, lines)` | `Event::pointer_down`, `pointer_up`, `pointer_drag`, `click`, `wheel` at the layout point of the cell (`Host::CELL` points per cell). `Event::click` also sends the move before the press |
| `sigil_kittest::input::{pointer_move, pointer_drag, pointer_down, pointer_up, click}(x, y)` | `Event::pointer_move`, `pointer_drag`, `pointer_down`, `pointer_up`, `click` with `Point::new(x, y)` |
| `input::key_event(key, physical, phase)`, `key_down(key, physical)`, `chord(key, physical, modifiers)` | `Event::key(key, phase).physical(physical)`, `Event::key_down(key).physical(physical)`, adding `.modifiers(modifiers)` for a chord |
| `input::press`, `type_char(text, physical)`, `type_text`, `tab`, `scroll_lines(x, y, dy)`, `window_blur`, `ime_event(event)` | `Event::press`, `Event::typed(text).physical(physical)`, `Event::type_text`, `Event::tab`, `Event::wheel(Point::new(x, y), dy)`, `Event::blur`, `Event::Ime(event)` |
| `app::TextTuning`, `set_text_tuning`, `text_tuning` | removed from the app API; no caller tuned glyphs. Tests keep `set_text_tuning` under feature `kittest` |
| `app::DefaultFonts`, `set_default_fonts` | `AppConfig::font_sans`, `font_sans_weight`, `font_mono`, `font_icon`, `font_display`, `font_fallback` |
| `animation_runtime::MotionPrefs::reduced()`, `MotionPrefs::set` | `animation::reduced_motion()`, `AppConfig::reduced_motion` |
| `sigil::animation_runtime::{MotionPrefs, active_count, presets, running_clocks, tick}` (feature `kittest`) | `sigil::animation::*`, under `kittest`, `#[doc(hidden)]`. The `sigil::animation_runtime` module is removed |
| `style::StyledTextEngine` (feature `kittest`) | removed; call `OwnedTextEngine::layout_paragraph_styled`, no trait import |
| `layout::TextEngineMeasure<OwnedTextEngine>` (feature `kittest`) | `layout::TextEngineMeasure` |
| `AsyncRuntime::drain` (feature `kittest`) | `AsyncRuntime::pump` |
| `app::HostCaps`, `Host::set_caps(HostCaps { hyperlinks })`, `Caps::host()` | `Host::hyperlinks(hyperlinks)` |
| `app::WindowOpts` | `AppConfig::overlay`, `frameless`, `anchor`, `user_zoom`, `min_size`, `resizable` |
| `reactive::effect_named` | `reactive::effect`. Named effects are crate-private |
| `tasks::on_message(rx, f)`, `on_message_or_close(rx, f, on_close)` | `tasks::on_message(rx, f, on_close)`; pass `\|\| {}` when nothing runs on close |
| `task_with`, `Task::run_with`, `Task<T, E, A>` | `task` and `Task<T, E>`; capture the argument in a signal the closure reads |
| `reactive::cancel_interval(id)` | `IntervalId::cancel` |
| `kernel::announce(text)`, `announce_assertive(text)` | `kernel::announce(text, Live::Polite)`, `announce(text, Live::Assertive)` |
| `scene::emit_scene(cx, root)`, `emit_scene_clipped(cx, root, clip)` | `scene::emit_scene(cx, root, clip)`; pass `None` for no clip |
| `list_row(selected, title, detail)`, `list_row_with(selected, title, slots)`, `ListRowSlots::row` | `list_row(selected, title, slots)`; `detail` is `ListRowSlots::status` |
| `image_rgba(w, h, rgba)`, `image_bytes(encoded)` | `ImageSource::from_rgba8(w, h, rgba).map(image)`, `ImageSource::decode(encoded).map(image)` |
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
| `style::StateVariants` (`rest`, `hover`, `focus`, `pressed`, `disabled_style`) | `style::States` (`rest`, `hover`, `focus`, `pressed`, `disabled_style`), implemented only for a builder that has state slots |
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
| `TextBuilder::label`, `description`, and `keyboard_shortcut` as `const fn` | the same methods, no longer `const` |
| `text(...).map_node` and `image(...).map_node` run at the call | they run when the node is built, after `layout`, `width`, `height`, and `grow` |
| a width or height hint ignores a size set through `layout` | it follows a size set through `layout` as well as through `width` and `height` |
| `*Builder` types a function returns (`CardBuilder`, `CheckboxBuilder`, `ChipBuilder`, `SwitchBuilder`, `StatusView`, and others) as nameable paths | the function returns them and their methods work; write `-> impl IntoNode` where a signature named one. `ButtonBuilder`, `ChartBuilder`, `DiffBuilder`, `IconBuilder`, `ImageBuilder`, `ModalBuilder`, `NodeBuilder`, `SparklineBuilder`, `TextBuilder`, `TextEditBuilder`, `TextFieldBuilder`, and `VirtualListBuilder` stay |
| bound traits `IntoPx`, `IntoIconSource`, `IntoScrollExtent`, `IntoList`, `DiffSource`, `FieldControl`, `MessageReceiver` as nameable paths | the bounds still apply; pass the same values |
| `animation::{Animator, Spring, SpringPreset, Tween}` | crate-private; `Animator` and `SpringPreset` stay public only under `kittest` |
| `animation::CubicBezier` | removed with `Easing::Bezier` |
| `PathNode` | `PathBuilder` |
| `.asymmetric(reason)` | `.allow_flaw(Flaw::Column, reason)` or `.allow_flaw(Flaw::Inset, reason)` |
| `TextStyle { font_size: 16.0, ..TextStyle::default() }` | `TextStyle::default().font_size(16.0)`; one setter per field |
| struct literals of `scene::DrawRect`, `scene::Shadow`, `events::Shortcut`, `events::PointerTilt` | these are `#[non_exhaustive]`; use their constructors |
| `Surface { key }` | `Surface::new(key)` |
| `FrameHooks { input: Some(Box::new(f)), .. }` | `FrameHooks::default().input(f)`, `.after_layout(f)`, `.paint(f)` |
| `terminal::Hooks { region_rows: Some(Box::new(f)), .. }` | `Hooks::default().region_rows(f)`, `.external_events(f)`, `.after_frame(f)` |
| `terminal::Access { focus_cursor, status }` | `Access::default().focus_cursor(..).status(..)` |
| struct literals of `TextFace`, `TextRun`, `Locale`, `terminal::Cell`, `terminal::Lines`, `scene::GlyphRun`, `ListState` | these are `#[non_exhaustive]`; read the fields, build with the constructor |
| `ProgressBuilder.indeterminate` field | `.indeterminate()` |
| `Theme` public fields (`theme.accent`, `theme.gray.step(n)`) | `theme.color(Token::Accent)`, `theme.scale(Tone::Neutral).step(n)`; write with `.gray(scale)`, `.accent_scale(scale)`, `.success(scale)`, `.warn(scale)`, `.error(scale)`, `.info(scale)` and `.token(name, value)` |
| `style::ScaleRole`, `Tone::scale_role` | `Tone` (`Gray` is `Tone::Neutral`, `Accent` is `Tone::Primary`) |
| `SelectionState` with `click`, `ctrl_click`, `shift_click`, `selected_indices` | `Selection` with `select`, `toggle`, `extend`, `indices`, `clear` |
| `TableState::select_only`, `toggle_selected`, `select_range_to`, `clear_selection` | `TableState::select`, `toggle`, `extend`, `clear` |
| diff `Selection` with public `hunks` and `lines` | `DiffSelection` with `hunks()` and `lines()` |
| `States::disabled_style` | `States::disabled_paint` |
| `widgets::close_top_escape` | `widgets::close_popup` |
| `MouseButtons::EMPTY` | `MouseButtons::empty()` |
| `list_row(selected, title, ListRowSlots { .. })` | `list_row(title).selected(selected)` with one setter per slot; `ListRowSlots` and `ListColumn` removed, `.column(text, width, ColumnAlign)` adds a column |
| `profile::BackendStatus { kind, fallback }` and its fields | `BackendStatus::new(backend, fallback)`, `backend()`, `fallback()` |
| `terminal::title`, `mouse_tracking`, `print_above`, `notify`, `progress`, `bell`, `activity`, `calibrate`, `suspend`, `caps`, `backdrop`, `pixels` free functions | the same names as methods of `terminal::Terminal::current()` |
| `terminal::focused()`, `terminal::set_pixels(on)` | `Terminal::is_focused()`, `Terminal::draw_pixels(on)` |
| field reads of `Patch`, `Hunk` and `Line` (`patch.hunks`, `hunk.path`, `line.old`) | getters: `patch.hunks()`, `hunk.path()`, `hunk.lines()`, `line.kind()`, `line.old_line()`, `line.new_line()` |
| struct literals and field reads of `SortKey`, `Highlight`, `Diagnostic`; field reads of `TreeNode` | `SortKey::new(column, direction)`, `Highlight::new(start, end, token)`, `Diagnostic::new(line, start, end, severity)` and a getter per field; `TreeNode` is built with `new` and its setters |
| public fields of `Date`, `Time`, `DateTime`, `MenuItem` and `WebFont` | `Date::year()`, `month()`, `day()`; `Time::hour()`, `minute()`, `second()`; `DateTime::date()`, `time()`; `WebFont::url()`; `MenuItem` and `WebFont` are built with `new` and their setters |
| `app::set_web_fallbacks(fonts)` | `AppConfig::web_fonts(fonts)` |
| `AppConfig::font_sans`, `font_mono`, `font_icon`, `font_display` | `AppConfig::font(FontFamily::Sans, bytes, face)` and the other families |
| `clipboard::clipboard()` | `clipboard::copy`, `paste`, `copy_image`; the handle is crate-private |
| `profile::tick_frame`, `profile::count_add(counter, n)` | `profile::end_frame`, `profile::count(counter, n)` |
| `ButtonBuilder::new` and the `new` of 15 other widget builders | the widget function: `button(label)`, `checkbox(checked)`, `slider(value, range)` |
| `animation::{Keyframe, Keyframes, AnimationDirection, AnimationFill, Playback, AnimatorStatus}`, `terminal::CellGrid`, `style::ThemeBuilder`, `widgets::FocusCaret`, `TextFieldBuilder::focus_caret`, `Widget::presence_spring`, `Widget::ime_caret` | removed; no caller used them |
| `Paint::{background_gradient, transform, transition, duration}`, `LayoutStyle::{align_content, box_sizing, flex_direction}`, `SceneBuilder::{draw_gradient, draw_pattern, draw_sprite, push_layer, draw_decoration, draw_image, draw_shadow, with_composite_layer}`, `DrawRect::border_radius`, `Theme::{locale, strings, gray, accent_scale, success, warn, error, info}`, `app::{set_ambiguous, set_cell_width}`, `style::render_style_for` | public only under `kittest` |
| `style::scale`, `style::{Locale, Strings, StyleAtom, StateStyle, StyleState, StyleVariant, StyleEngine, RenderStyle, Transform2D, TransitionId, MillisU16, BgGradient}` and the other style value types | public only under `kittest` or crate-private; `Token::scale` and `Tone::scale_role` still work without naming `ScaleRole` |
| `scene::{DrawGradient, DrawImage, DrawPattern, DrawSprite, DrawDecoration, LayerEffect, LayerScope, LinearGradient, GradientStop, CornerRadii, StrokeAlign, StrokeStyle}` and the other command payloads | public only under `kittest` or crate-private; `SceneCommand` fields keep these types |
| `AccessibleName`, `FocusScopeKind`, `NodeExtensions`, `ImageCrop`, `icons::{IconBaker, BakedIcon}` (now `icons::bake`), `events::FileDropEvent`, `reactive::DrawPhase` | public only under `kittest`; `.label("text")` and `.file_drop(\|event\| ...)` work without the names |
| `app::{FrameOutput, FrameLayoutHook, Ambiguous, CellWidth}`, `terminal::{AfterFrame, EventPump, ClipRoute, Erase, Glyphs, Notices, Placement}`, `profile::{Attribution, PhaseTime, CounterValue}`, `tasks::TaskHandle`, `widgets::{ButtonGlyph, LineId, LineKind, PathCmd, PathFill, TreeRow, IconSource}`, `events::PointerTilt` | no public path; functions return them and fields keep them |
| `style::palette::Palette` | crate-private. Use `Theme` |
| `geometry::Scale`, `Cx::scale() -> Scale`, `Cx::set_scale(Scale)` | `Cx::scale() -> f32`, `Cx::set_scale(f32)`. `Scale` is public only under `kittest` |
| `widgets::ColumnLayout`, `TableState::column_layout` | `TableState::hide_column`, `show_column`, `visible_columns`, `column_width`; the layout is crate-private |
| `Shadow::kind`, `DrawRect::stroke` fields | public only under `kittest` |
| `reactive::{Runtime, next_interval_deadline, debug}`, `style::unknown_class_warning_count`, `scene::{paint_backdrop, shape_glyph_run, Stroke}`, `terminal::{map_key, map_mouse, cell_point}`, `A11yProps`, `Cx::new_node`, `clipboard::NoopClipboard`, `DrawRect::stroke`, `Shadow::with_kind` | public only under `kittest` |
| `style::{variant, Variant, is_typography_atom}`, `Theme::from_palette`, `terminal::{install_clipboard, blocks_window, png_window, sextants_window, detect_mux}`, `widgets::{listbox, ListModel, language_for_path, text_runs, ToastEntry}`, `app::{cell_text_lines, ellipsize_cells, ellipsize_cells_start, fit_cells}`, `tasks::panic_message`, `style::font_family_hint` | no public path; `Caps` reports the multiplexer and `ToastQueue` owns its entries |
| `widgets::to_hex(color)` | `Color::to_hex(self)` |
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
| --- | --- |
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
| `Paint::bg(impl Into<ColorAtom>)` | `Paint::background(impl Into<Background>)`; `Token`, `Color`, and `ColorAtom` convert with `From` |
| `sigil_sign::SignError`, `sigil_sign::PackError` | `sigil_sign::Error`; branch on `error.kind()`, `error.is_password_required()`, `error.path()` |
| `sigil_sign::pack_cli` returning `Result<(), String>` | `Result<(), sigil_sign::Error>` |
| `AppBundle { .. }`, `ResourceCopy { .. }`, `CodeSign { .. }`, `Dmg { .. }` struct literals | `AppBundle::new(name, executable)`, `ResourceCopy::new(from, to)`, `CodeSign::default()`, `Dmg::new(app, volume)` plus setters named for the fields |
| `sigil_kittest::Error::{NoAdapter, Device, Layout, Render, Readback, Unsettled}` | `error.kind() == ErrorKind::..`, `error.is_no_adapter()`, `error.is_unsettled()` |
| `Harness::present` error type `sigil::Error` | `sigil_kittest::Error` |
| `gpu::Capture { .. }` | `Capture::new(viewport, size_px).scale(..).clear(..)` |
| `GeneratedScene` fields | getters of the same names |
| `Host::route(event)` returning `()` | returns `bool`: whether a key press was consumed |
| `esc_exits(true)`: Escape exited before the UI saw it | exits only when nothing consumed the press |
| terminal `quit_on_ctrl_c(true)`: Ctrl+C exited before routing | copies a selection or reaches a focused `terminal()` first |
| `Metrics::padding(size)`, `Metrics::gap(size)` | `Metrics::control_padding(size)`, `Metrics::control_gap(size)` |
| `Theme::padding(size)`, `Theme::gap(size)` | `Theme::control_padding(size)`, `Theme::control_gap(size)` |
| exhaustive `match` on `Easing`, `HostMode`, `ObjectFit`, `Dimension`, `Display`, `Position`, `TrackSize`, `SceneCommand`, `Clip`, `Elevation`, `Radius`, `ControlSize`, `ClipShape`, `ColorAtom`, `MaskImage`, `FontFamily`, `WebFontScript`, `Change`, `Side`, `RowTone`, `SparkMark`, `SparkFill`, `ColWidth`, `ColAlign`, `Density`, `SubmitKey`, `ScrollAnchor`, `SelectionMode`, or terminal `ColorDepth`, `Keyboard`, `Borders`, `Cursor`, `Notify`, `MouseTracking`, `Activity`, `Underline` | add a `_` arm; all are `#[non_exhaustive]` |
| under feature `kittest`: crate-root `NodeType`, `AccessibleName`, `FocusScopeKind`, `ImageCrop`, `NodeExtensions`, `CaretGeometry`, `TextMeasure`, `HitPosition`, `TextAffinity` (use `sigil::kernel`, `sigil::layout`, `sigil::text`); `animation::{KeyframesId, linear_channel_to_srgb}`, `profile::Span`, `scene::DrawPath`, `style::{BorderStyle, GradientShape, FilterAtom, MaskAtom, MillisU16, PaintStyle, SpaceAtom, SpaceStyle, Backface, StopSlot, TextRun, text_style_for}`, `terminal::{base64, detect}`, `widgets::interaction`, `widgets::RadioGroupBuilder`, `TextBuilder::runs`, `Date::parse_in`, `Date::render_in` | crate-private |
| `Dimension` | `Length` |
| `PathNode::fill` and `stroke` took an unnameable `PathColor` | they take `impl Into<ColorAtom>`: pass a `Color`, a `Token` or a `ColorAtom` |
| `Rgba8` | `Color` |
| `ColorAtom::Rgba8(..)` | `ColorAtom::Color(..)` |
| `sigil::style::Color` (f32 RGBA) | `sigil::style::Rgba`, public only under `kittest`, `#[doc(hidden)]`; app code uses `Color` or `ColorAtom` |
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
| `LayoutStyle`, `Widget::layout` | `LayoutStyle` and the raw `.layout(LayoutStyle)` setter are public only under feature `kittest`; `Widget` no longer has a `layout` method. Use the container functions and the `Widget` setters |
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
| `ScaleRole::Warn` | `ScaleRole::Warning` |
| `ButtonBuilder::shortcut(text)`, `MenuItem::shortcut(text)`, the `MenuItem::shortcut` field | `ButtonBuilder::hint(text)`, `MenuItem::hint(text)`, the `MenuItem::hint` field; `Widget::shortcut(shortcut, handler)` still binds a key |
| `divider_horizontal()`, `divider_vertical()` | `divider(Orientation::Horizontal)`, `divider(Orientation::Vertical)` |
| `Paint::line_height_px(px)`, `Paint::letter_spacing_em(em)` | `Paint::line_height(px)`, `Paint::letter_spacing(em)`; the units are unchanged |
| `Paint::mix_blend(BlendMode)` | `Paint::blend(BlendMode)` |
| `ColorAtom` in `sigil::prelude` | `sigil::style::ColorAtom`; setters still take `Token` and `Color` through `Into<ColorAtom>` |
| `set_theme(theme)`, `sigil::app::set_theme(theme)` | `theme.apply()` |

### Changed

- `panel` and `PanelBuilder` are removed; `card` takes a title. Migration: `panel().title("T").child(body)` becomes `card(body).title("T")`, and `.elevation(0)` becomes `.layer(Layer::Page)`. A card has 12 px padding where a panel had 16 px; chain `.layout(LayoutStyle::default().padding(Space::Lg))` to keep it.
- Names shortened to one word each. Migration: `derived(f)` becomes `memo(f)`; `IntervalId` becomes `Timer`; `ModifiersState` becomes `Modifiers` and `ModifiersState::CONTROL` becomes `Modifiers::CTRL`; `.on_added(f)` becomes `.added(f)`; `.animate(keyframes)` becomes `.animation(keyframes)`; `signal_writer(s)` becomes `s.writer()`.
- Setters and handlers are the field or event name with no prefix. Migration: `on_cleanup(f)` becomes `cleanup(f)`; `AppConfig::on_close`/`on_exit` become `close`/`exit`; `esc_exits`/`quit_on_ctrl_c` become `exit_on_esc`/`exit_on_ctrl_c`; `request_exit()` becomes `exit()`; `Locale::with_date`/`with_group_size`/`with_zero` become `date`/`group`/`zero`; `terminal::set_title`/`set_mouse_tracking` become `title`/`mouse_tracking`; `Screen::set_access`/`set_dt` become `access`/`dt`; `AsyncRuntime::set_waker` becomes `waker`.
- More prefixes dropped. Migration: `request_focus(id)` becomes `id.focus()`; `tasks::on_message` becomes `tasks::listen`; `SceneBuilder::with_clip`/`with_transform`/`with_layer`/`with_composite_layer` become `clip`/`transform`/`layer`/`composite`; `Shadow::with_kind` becomes `kind`; `with_on_solid` becomes `on_solid`; `Node::get_extension`/`get_extension_mut` become `extension`/`extension_mut`.
- One setter and one getter per job. Migration: `Harness::get_by_role(role, name)` becomes `get((role, name))` and `get_by_text(text)` becomes `get(text)`; `get` also still takes a `Query`. `StateStyle::with_hovered(atoms)` and the other four become `slot(StateSlot::Hovered, atoms)`, and `hovered()` and the other four readers become `get(StateSlot::Hovered)`. The layout constraint `with_fixed_width(w)` becomes `width(w)`; `with_min_content_width` and `with_max_content_width` had no callers and are removed.
- Setters and constructors lose their prefix where no getter has the name. Migration: `events::request_window_command(command)` becomes `events::window_command(command)`, `Cx::add_child(parent, child)` becomes `Cx::append(parent, child)`, `LayoutEngine::with_text_measure(measure)` becomes `LayoutEngine::new(measure)`, `app::set_ambiguous(width)` becomes `app::ambiguous(width)`, `layout::set_text_geometry(measure)` becomes `layout::text_geometry(measure)`, `testing::with_cx` and `with_runtime` become `testing::cx` and `testing::runtime`, and `Screen::with_caps(cols, rows, caps)` becomes `Screen::new(cols, rows)` followed by `set_caps(caps)`. A setter keeps `set_` only where a getter of the same name exists, such as `Cell::text` and `Cell::set_text`.
- One focus ring setter. Migration: `focus_ring_color(c)`, `focus_ring_width(w)`, `focus_ring_offset(o)` and `focus_ring_opacity(a)` become `focus_ring(FocusRing::default().color(c).width(w).offset(o).opacity(a))`; `focus_ring(visible)` still takes a `Signal<bool>`, and `FocusRing::from(visible)` adds the other settings to it. `FocusRing` is in the prelude.
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
- The `icon_svg` function is removed. Migration: `icon_svg(source)` becomes `icon(Glyph::svg(source))`. `Glyph` has a new `Svg` variant and is in the prelude.
- Two `sigil_kittest::Harness` values on one thread each get the results of their own tasks. Before, the second harness took the thread's task dispatcher and the first one's `spawn`, `task` and `signal_writer` results never arrived.
- A `SignalWriter` writes for as long as its signal is alive, and `signal_writer` works outside a reactive scope. Before, a writer made in a scope that later ended dropped every write without an error, and a write to a disposed signal panicked the UI thread; that write is now dropped.
- A scroll moves one shared offset that `LayoutResult::rect` adds when read; it no longer rewrites the layout of every node under a `scroll`, a `virtual_list` or a `position_reactive` node. `rect` returns the same on-screen rectangle as before. `LayoutResult::rect` is no longer `const`, and `ReactivePosition` is `Clone` but no longer `Copy`.
- A node gets its own reactive owner only when its build asks for the current owner, or when the node has event handlers or a style binding. Before, every node got one. A node without its own owner runs later callbacks under the nearest owner above it. Each row of a reactive child list still gets an owner.
- `Host::frame_due()` returns whether a frame has work to do without new input: a signal write, an animation, a clock, or a timer that is due. An embed host calls it to skip `Host::frame` while the UI is idle.
- `terminal::Cell` is `Copy` and holds its text and hyperlink in 30 bytes. Text longer than seven bytes and each hyperlink are stored once for the life of the process, up to 65,534 of each and 2,048 bytes per entry; past that a cell shows U+FFFD and a link is dropped.
- `A11yProps` keeps `role`, `name`, `focusable`, `traps_tab`, `hittable` and `focus_scope`; its other fields moved to `A11yDetail`, which `A11yProps` dereferences to, so `props.checked` still reads and assigns. Migration: build one with `A11yProps::default()` and assign the fields; a struct literal can no longer name the moved fields. The `NodeBuilder` setters for the moved fields are no longer `const fn`.
- The minimum supported Rust version is 1.96. Before, it was 1.95.
- `Combobox::placeholder` and `TagInputBuilder::placeholder` take `impl Into<String>`. Before, they took `&'static str`.
- `TextFieldBuilder::change`, `submit` and `blur` add a callback on each call. Before, a second call replaced the first, so `number_field(..).text_field().submit(f)` dropped the clamp on commit.
- `range_slider` returns `SliderBuilder<RangeInclusive<f32>>`, and its `.change` gets the whole range after either thumb moves. Before, it got one thumb's value.
- `Theme::token(name, value)` sets a theme token and takes a `Color`. It replaces `Theme::register_token` and `Theme::with_token`.
- A table cell edit commits and closes when focus leaves the editor. Before, it stayed open.
- `Theme` setters are named for their fields: `with_gray_scale`, `with_accent_scale`, `with_success_scale`, `with_warn_scale`, `with_error_scale`, `with_info_scale`, `with_direction`, `with_locale` and `with_strings` become `gray`, `accent_scale`, `success`, `warn`, `error`, `info`, `direction`, `locale` and `strings`.
- `Theme::color` takes a `Token`, a `Color` or a `ColorAtom`, and replaces `Theme::resolve_color`. `Theme::text_muted`, `surface_subtle` and `border_strong` are removed; read `Theme::color(Token::OnSurfaceMuted)`, `Token::SurfaceSubtle` and `Token::BorderStrong`, which also follow token overrides.
- A `combobox` keeps focus in its input after the user picks an option. Before, a pick rebuilt the input and focus moved to the next control.
- A parsed field (`date_field`, `time_field`, `number_field`, `color_field`) and a `text_field` with `invalid_when` report the invalid state to screen readers, not only in paint. `field` no longer clears it when there is no `.error`, and `invalid_when` adds a condition on each call instead of replacing the last one.
- The clear button, the tag remove button, the range slider thumbs and the 12-hour day periods read their text from `StringKey::{Clear, RemoveItem, Minimum, Maximum, Am, Pm}`, so a `Strings` table can translate them. `Locale::format_time` and `parse_time` take the `Strings` table.
- `DateFieldBuilder` implements `FieldControl`, so `field(..).error(..)` accepts it.
- `Locale` has a new `hour12` field. Migration: a `Locale { .. }` struct literal adds `hour12: false` for a 24-hour clock.
- `virtual_list` with a fixed `viewport_height` and `follow_end`, `initial_scroll` or a scroll to a row no longer paints a blank viewport on first frame.
- `open` and `link` URLs work in terminal-only builds (feature `terminal`).
- Parsed fields (`date_field`, `number_field`, `color_field`, `time_field`) no longer reformat the text while the user types; before, a date field turned "2026-08-1" into "2026-08-01" under the caret. The text is rewritten to the stored value when the user presses Enter or leaves the field, when the app sets the value, and when the locale changes, so "500" in a `0..=100` field shows "100" and "1.50" shows "1.5".
- `splitter` and `tabs` free their drag state when they unmount. Before, each mount leaked it.
- `AppConfig::on_exit` runs on the terminal host and in builds without the `desktop` feature. Before, the terminal host never called it.
- A text selection belongs to the host where it was made. Before, a frame in a second window or host could clear it or highlight text in the wrong tree.
- Engine internals left the public API. `Cx::dirty_all`, `has_specific_dirty`, `get_mut_untracked`, `instance_id`, `root_owner`, `roots`, `host_mode` and `describe`, `Node::attach_untracked`, `get_extension_mut_untracked` and the `owner` field, `PointerEvent::set_captured` and `capture_requested`, `SceneBuilder::draw_glyph_run_for_node`, `Signal::try_peek` and `style::cell` are crate-private. `Cx::dirty`, `clear_dirty`, `mark_mutated`, `mutation_generation`, `scale`, `set_scale`, `first_child`, `flush_announcements`, `flush_reactive_children`, `dump_tree`, `Node::version`, `Host::set_hyperlinks`, `reset_emit`, `paint_cells` and `SceneBuilder::pop_layer` are public only under feature `kittest`. `Host::take_exit`, `Host::hit_path` and `Node::text_cursor` are removed; a frame's exit request is `FrameOutput::exit`.
- Every type that a public function takes or returns can be named. `sigil::widgets` exports each widget builder and its value types (`Glyph`, `FuzzyMatch`, `LineId`, `LineKind`, `DiffSource`, `FieldControl`, `IntoList`, `NumericKind`). `sigil::style` exports `ColorScale`, `ScaleRole`, `Locale`, `DateOrder`, `Strings`, `StringKey`, `TextLineHeight`, `TextDecoration`, `TextRuns`, `TextRun`, `TextFace` and `StateStyleBuilder`. `sigil::scene` exports `SceneBuilder`, `CommandScene`, `DrawRect`, `GlyphRun`, `Wash`, `StrokeCap`, `StrokeJoin`, `BlendMode` and `Shadow`. The root exports `IntoPx`, and `sigil::geometry` exports `Scale`. `FrameOutput`, `FileDropEvent`, `PointerTilt`, `MessageReceiver`, `Span`, `Subscription` and the terminal `ClipRoute`, `Erase`, `Notices` and `Glyphs` are public. `FrameOutput` and `FileDropEvent` are `#[non_exhaustive]`.
- Every builder type is `#[must_use]`.
- `ButtonGlyph`, `IconSource` and `IntoIconSource` are one type, `widgets::Glyph` (`#[non_exhaustive]`, variants `Text`, `Icon` and `Svg`). `icon`, `button` and `TreeNode::leading` take `impl Into<Glyph>`, so the same values work.
- `ScrollBuilder::extent` takes `impl Reactive<f32>`: a number, a `Signal<f32>`, or a closure. `IntoScrollExtent` is removed.
- `GlyphRun` no longer derefs to `SharedGlyph`. Read a run with `text()`, `color()`, `style()`, `opacity()` and `tone()`. `SharedGlyph` is crate-private. Migration: `run.text.clone()` becomes `run.text().to_owned()`.
- `Theme::tokens` is crate-private, and `TokenTable` and `TokenLayer` are no longer exported. Read a token with `Theme::color` and set one with `Theme::token`. Writing the field directly left `accent` and its scale stale. A `Theme` is no longer built with `..Theme::default()` outside the crate; start from `Theme::default()` and set fields.
- `app::FrameInputHook`, `FrameLayoutHook` and `FramePaintHook` are public, like `FrameHooks` that names them. `render_style_for` (feature `kittest`) no longer takes an unused node id.
- `terminal::Capabilities` and `terminal::Notices` fields are private. Read them with getters of the same names, for example `caps.backdrop()`. `Screen::with_caps` and `Screen::set_caps` are public only under feature `kittest`; a host gets its capabilities from `Capabilities::detect` or `probe`. `terminal::ClipRoute` and `terminal::Fills` are `#[non_exhaustive]`.
- `StyleAtom` and its payload types, the scene command types other than those above, `emit_scene`, `CommandScene::commands`, and the terminal `map_key` and `map_mouse` are public only under feature `kittest`.
- `kernel::Patch` is renamed `NodePatch`. `LayoutStyle::pane`, `StyleAtom::UnknownThemeRef`, `unsupported_style_warning_count_for` and `Hotspot::RowRebuild` are removed. `AppConfig::on_exit` builds only with the `desktop` or `terminal` feature.
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
- Layout is set with the container functions `row`, `column`, `grid`, and `container` and with the layout setters on `Widget` (`.gap(..)`, `.padding_x(..)`, `.fill()`, `.min_width(..)`, `.align_items(..)`, and the rest). `LayoutStyle` and the raw `.layout(LayoutStyle)` setter are public only under feature `kittest`, and `Widget` no longer has a `layout` method; the migration table maps each constructor and method.

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
- `tag_input(options, selected)`: a filterable multi-select whose chosen options show as removable tags. Backspace in an empty input removes the last tag. `.disabled(..)` also disables the remove buttons.
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
- Keyframe animations: `Paint::animate(Keyframes)`, with `Keyframes` stops, per-stop easing, iterations, `reverse` and `alternate`, and the Tailwind `spin`, `ping`, `pulse` and `bounce` presets. `Property` names the animated value.
- CSS-style transitions: `Paint::transition(Transition)`, with `Transition::new`, `all`, `colors`, `opacity` and `transform`, and the `duration`, `ease` and `delay` setters.
- Vector paths draw on the GPU through `vello_gpu` and on the CPU through `vello_cpu` instead of a tiny-skia bitmap. `PathNode` gains `even_odd`, `cap(StrokeCap)`, `join(StrokeJoin)` and `dash`; `scene::StrokeCap` and `scene::StrokeJoin` are new.
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
- `SplitterBuilder::proportional` keeps pane shares when the parent resizes, and `SplitterBuilder::settle_bind` reports the sizes once when a drag or key move ends.
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
- Flexbox and grid layout through `row`, `column`, `grid`, and the `Widget` layout setters, with pixel sizes and text units `rows`, `cols`, and `em`.
- Keyboard, pointer, scroll, paste, and gesture handlers through `Widget`. `Shortcut` parses and displays key combinations, with `Shortcut::primary` selecting Command or Control for the platform.

#### Widgets

- Push buttons through `button`, with independent tone and fill choices, disabled states, and activation callbacks.
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
- Selectable text through `Widget::selectable_text`, with clipboard-result callbacks through `Widget::selection_copy`.
- Syntax highlighting through `highlight` for Rust, shell, TOML, JSON, Python, JavaScript, TypeScript, Go, C/C++, YAML, CSS, HTML, SQL, Markdown, and diffs.

#### Animation

- Spring and tween style transitions, chosen by `Easing`.
- Style transitions and animated appearance or disappearance through `transition` and `presence`.
- Continuous motion through `animation::clock`, a reactive elapsed-time clock that advances on painted frames.
- Reduced-motion preferences through `animation::reduced_motion`, with OS defaults and an `AppConfig::reduced_motion` override.

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
