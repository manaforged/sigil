# Module `sigil::style`

| Item | Kind | Description |
| --- | --- | --- |
| [`ToneContext`](#tonecontext) | struct |  |
| [`FocusRing`](#focusring) | struct |  |
| [`Rgba8`](#rgba8) | struct |  |
| [`Theme`](#theme) | struct |  |
| [`BgGradient`](#bggradient) | struct |  |
| [`Color`](#color) | struct |  |
| [`GradientStopKey`](#gradientstopkey) | struct |  |
| [`Locale`](#locale) | struct |  |
| [`Metrics`](#metrics) | struct |  |
| [`MillisU16`](#millisu16) | struct |  |
| [`Px`](#px) | struct |  |
| [`RenderStyle`](#renderstyle) | struct |  |
| [`StateStyle`](#statestyle) | struct |  |
| [`StateStyleBuilder`](#statestylebuilder) | struct |  |
| [`Strings`](#strings) | struct |  |
| [`StyleBinding`](#stylebinding) | struct |  |
| [`StyleEngine`](#styleengine) | struct |  |
| [`StyleOverlay`](#styleoverlay) | struct |  |
| [`StyleReactiveEffect`](#stylereactiveeffect) | struct |  |
| [`StyleState`](#stylestate) | struct |  |
| [`StyleVariant`](#stylevariant) | struct |  |
| [`TextFace`](#textface) | struct |  |
| [`TextRun`](#textrun) | struct |  |
| [`TextRuns`](#textruns) | struct |  |
| [`TextStyle`](#textstyle) | struct |  |
| [`ThemeBuilder`](#themebuilder) | struct |  |
| [`TokenLayer`](#tokenlayer) | struct |  |
| [`TokenTable`](#tokentable) | struct |  |
| [`Transform2D`](#transform2d) | struct |  |
| [`TransitionId`](#transitionid) | struct |  |
| [`ColorAtom`](#coloratom) | enum |  |
| [`ColorOrToken`](#colorortoken) | enum |  |
| [`ControlSize`](#controlsize) | enum |  |
| [`Elevation`](#elevation) | enum |  |
| [`FontFamily`](#fontfamily) | enum |  |
| [`Radius`](#radius) | enum |  |
| [`Token`](#token) | enum |  |
| [`ColorKey`](#colorkey) | enum |  |
| [`Tone`](#tone) | enum |  |
| [`AlignKind`](#alignkind) | enum |  |
| [`ColorAtomKind`](#coloratomkind) | enum |  |
| [`DateOrder`](#dateorder) | enum |  |
| [`Dimension`](#dimension) | enum |  |
| [`Direction`](#direction) | enum |  |
| [`DisplayKind`](#displaykind) | enum |  |
| [`FlexDir`](#flexdir) | enum |  |
| [`FlexWrapKind`](#flexwrapkind) | enum |  |
| [`GradientAxis`](#gradientaxis) | enum |  |
| [`JustifyKind`](#justifykind) | enum |  |
| [`LengthPercentage`](#lengthpercentage) | enum |  |
| [`NumericAtomKind`](#numericatomkind) | enum |  |
| [`OverflowKind`](#overflowkind) | enum |  |
| [`StateSlot`](#stateslot) | enum |  |
| [`StringKey`](#stringkey) | enum |  |
| [`StyleAtom`](#styleatom) | enum |  |
| [`TextDecoration`](#textdecoration) | enum |  |
| [`TextLineHeight`](#textlineheight) | enum |  |
| [`ThemeValue`](#themevalue) | enum |  |
| [`AnimatorOverlay`](#animatoroverlay) | trait |  |
| [`NodeStyleExt`](#nodestyleext) | trait |  |
| [`Paint`](#paint) | trait |  |
| [`StateVariants`](#statevariants) | trait |  |
| [`StyledTextEngine`](#styledtextengine) | trait |  |
| [`font_family_hint`](#font_family_hint) | fn |  |
| [`grid`](#grid) | fn |  |
| [`is_typography_atom`](#is_typography_atom) | fn |  |
| [`render_style_for`](#render_style_for) | fn |  |
| [`space`](#space) | fn |  |
| [`text_style_for`](#text_style_for) | fn |  |
| [`unknown_class_warning_count`](#unknown_class_warning_count) | fn |  |
| [`ComponentId`](#componentid) | type |  |
| [`ReactiveStateSlot`](#reactivestateslot) | type |  |
| [`ReactiveStyleAtoms`](#reactivestyleatoms) | type |  |
| [`BORDER_WIDTH`](#border_width) | const |  |
| [`DISABLED_OPACITY`](#disabled_opacity) | const |  |
| [`GHOST_VARIANT`](#ghost_variant) | const |  |
| [`ICON_SIZE`](#icon_size) | const |  |
| [`OVERLAY_SHADOW`](#overlay_shadow) | const |  |
| [`SELECTION_FILL`](#selection_fill) | const |  |
| [`SOFT_VARIANT`](#soft_variant) | const |  |
| [`SOLID_VARIANT`](#solid_variant) | const |  |
| [`SPACING_STEP`](#spacing_step) | const |  |

## Structs

### `ToneContext`

**Methods**

| Method | Description |
| --- | --- |
| <code>const new(tone: <a href="#tone">Tone</a>) -&gt; Self</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>tone: <a href="#tone">Tone</a></code> |  |

**Trait implementations:** <code>From&lt;<a href="#tone">Tone</a>&gt;</code>, <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `FocusRing`

**Methods**

| Method | Description |
| --- | --- |
| <code>const color(self, color: <a href="#rgba8">Rgba8</a>) -&gt; Self</code> |  |
| <code>const color_atom(&amp;self) -&gt; <a href="#coloratom">ColorAtom</a></code> |  |
| <code>const new(visible: <a href="sigil-reactive.html#signal">Signal</a>&lt;bool&gt;) -&gt; Self</code> |  |
| <code>const offset(self, offset: f32) -&gt; Self</code> |  |
| <code>offset_px(&amp;self, metrics: <a href="#metrics">Metrics</a>) -&gt; f32</code> |  |
| <code>const opacity(self, opacity: u16) -&gt; Self</code> |  |
| <code>const opacity_value(&amp;self) -&gt; u16</code> |  |
| <code>const static_visible(visible: bool) -&gt; Self</code> |  |
| <code>stroke_width(&amp;self, metrics: <a href="#metrics">Metrics</a>) -&gt; f32</code> |  |
| <code>const theme_color(self, token: <a href="#token">Token</a>) -&gt; Self</code> |  |
| <code>visible(&amp;self) -&gt; bool</code> |  |
| <code>const width(self, width: f32) -&gt; Self</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Debug</code>


### `Rgba8`

**Fields**

| Field | Description |
| --- | --- |
| <code>a: u8</code> |  |
| <code>b: u8</code> |  |
| <code>g: u8</code> |  |
| <code>r: u8</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Theme`

**Methods**

| Method | Description |
| --- | --- |
| <code>const border_strong(&amp;self) -&gt; <a href="#rgba8">Rgba8</a></code> |  |
| <code>const scale(&amp;self, role: <a href="sigil-style-scale.html#scalerole">ScaleRole</a>) -&gt; &amp;<a href="sigil-style-scale.html#colorscale">ColorScale</a></code> |  |
| <code>const surface_subtle(&amp;self) -&gt; <a href="#rgba8">Rgba8</a></code> |  |
| <code>const text_muted(&amp;self) -&gt; <a href="#rgba8">Rgba8</a></code> |  |
| <code>const with_accent_scale(self, scale: <a href="sigil-style-scale.html#colorscale">ColorScale</a>) -&gt; Self</code> |  |
| <code>const with_direction(self, direction: <a href="#direction">Direction</a>) -&gt; Self</code> |  |
| <code>const with_error_scale(self, scale: <a href="sigil-style-scale.html#colorscale">ColorScale</a>) -&gt; Self</code> |  |
| <code>const with_gray_scale(self, scale: <a href="sigil-style-scale.html#colorscale">ColorScale</a>) -&gt; Self</code> |  |
| <code>const with_info_scale(self, scale: <a href="sigil-style-scale.html#colorscale">ColorScale</a>) -&gt; Self</code> |  |
| <code>const with_locale(self, locale: <a href="#locale">Locale</a>) -&gt; Self</code> |  |
| <code>with_strings(self, strings: <a href="#strings">Strings</a>) -&gt; Self</code> |  |
| <code>const with_success_scale(self, scale: <a href="sigil-style-scale.html#colorscale">ColorScale</a>) -&gt; Self</code> |  |
| <code>const with_warn_scale(self, scale: <a href="sigil-style-scale.html#colorscale">ColorScale</a>) -&gt; Self</code> |  |
| <code>color(&amp;self, token: <a href="#token">Token</a>) -&gt; <a href="#rgba8">Rgba8</a></code> |  |
| <code>register_token(&amp;mut self, name: &amp;str, value: <a href="#themevalue">ThemeValue</a>) -&gt; Option&lt;<a href="#themevalue">ThemeValue</a>&gt;</code> |  |
| <code>resolve_color(&amp;self, key: <a href="#colorkey">ColorKey</a>) -&gt; <a href="#rgba8">Rgba8</a></code> |  |
| <code>with_theme_tokens(self, tokens: &amp;Map&lt;&amp;'static str, <a href="#themevalue">ThemeValue</a>&gt;) -&gt; Self</code> |  |
| <code>with_token(self, name: &amp;str, value: <a href="#themevalue">ThemeValue</a>) -&gt; Self</code> |  |
| <code>console_dark() -&gt; Self</code> |  |
| <code>control_height(&amp;self, size: <a href="#controlsize">ControlSize</a>) -&gt; <a href="#dimension">Dimension</a></code> |  |
| <code>control_width(&amp;self, size: <a href="#controlsize">ControlSize</a>) -&gt; <a href="#dimension">Dimension</a></code> |  |
| <code>from_palette(palette: <a href="sigil-style-palette.html#palette">Palette</a>, variant: <a href="sigil-style-variant.html#variant">Variant</a>) -&gt; Self</code> |  |
| <code>gap(&amp;self, size: <a href="#controlsize">ControlSize</a>) -&gt; <a href="#lengthpercentage">LengthPercentage</a></code> |  |
| <code>normal() -&gt; Self</code> |  |
| <code>padding(&amp;self, size: <a href="#controlsize">ControlSize</a>) -&gt; <a href="#lengthpercentage">LengthPercentage</a></code> |  |
| <code>const radius(&amp;self, size: <a href="#controlsize">ControlSize</a>) -&gt; <a href="#lengthpercentage">LengthPercentage</a></code> |  |
| <code>terminal() -&gt; Self</code> |  |
| <code>dark() -&gt; Self</code> |  |
| <code>light() -&gt; Self</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>accent: <a href="#rgba8">Rgba8</a></code> |  |
| <code>accent_hover: <a href="#rgba8">Rgba8</a></code> |  |
| <code>accent_scale: <a href="sigil-style-scale.html#colorscale">ColorScale</a></code> |  |
| <code>border: <a href="#rgba8">Rgba8</a></code> |  |
| <code>direction: <a href="#direction">Direction</a></code> |  |
| <code>error: <a href="sigil-style-scale.html#colorscale">ColorScale</a></code> |  |
| <code>gray: <a href="sigil-style-scale.html#colorscale">ColorScale</a></code> |  |
| <code>info: <a href="sigil-style-scale.html#colorscale">ColorScale</a></code> |  |
| <code>locale: <a href="#locale">Locale</a></code> |  |
| <code>metrics: <a href="#metrics">Metrics</a></code> |  |
| <code>missing_token_fallback: <a href="#rgba8">Rgba8</a></code> |  |
| <code>on_accent: <a href="#rgba8">Rgba8</a></code> |  |
| <code>on_solid: <a href="#rgba8">Rgba8</a></code> |  |
| <code>on_surface: <a href="#rgba8">Rgba8</a></code> |  |
| <code>strings: <a href="#strings">Strings</a></code> |  |
| <code>success: <a href="sigil-style-scale.html#colorscale">ColorScale</a></code> |  |
| <code>surface: <a href="#rgba8">Rgba8</a></code> |  |
| <code>surface_hover: <a href="#rgba8">Rgba8</a></code> |  |
| <code>surface_pressed: <a href="#rgba8">Rgba8</a></code> |  |
| <code>surface_pressed_strong: <a href="#rgba8">Rgba8</a></code> |  |
| <code>tokens: <a href="#tokentable">TokenTable</a></code> |  |
| <code>warn: <a href="sigil-style-scale.html#colorscale">ColorScale</a></code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


### `BgGradient`

**Methods**

| Method | Description |
| --- | --- |
| <code>const new(axis: <a href="#gradientaxis">GradientAxis</a>, stops: &amp;'static [<a href="#gradientstopkey">GradientStopKey</a>]) -&gt; Self</code> |  |
| <code>owned(axis: <a href="#gradientaxis">GradientAxis</a>, stops: Vec&lt;<a href="#gradientstopkey">GradientStopKey</a>&gt;) -&gt; Self</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>axis: <a href="#gradientaxis">GradientAxis</a></code> |  |
| <code>stops: &amp;'static [<a href="#gradientstopkey">GradientStopKey</a>]</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Color`

**Methods**

| Method | Description |
| --- | --- |
| <code>const BLACK: Self</code> |  |
| <code>const TRANSPARENT: Self</code> |  |
| <code>const WHITE: Self</code> |  |
| <code>const rgba(r: f32, g: f32, b: f32, a: f32) -&gt; Self</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>a: f32</code> |  |
| <code>b: f32</code> |  |
| <code>g: f32</code> |  |
| <code>r: f32</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `GradientStopKey`

**Methods**

| Method | Description |
| --- | --- |
| <code>const new(offset: u16, color: <a href="#coloratom">ColorAtom</a>) -&gt; Self</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>color: <a href="#coloratom">ColorAtom</a></code> |  |
| <code>offset: u16</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Locale`

**Methods**

| Method | Description |
| --- | --- |
| <code>const ar() -&gt; Self</code> |  |
| <code>const de() -&gt; Self</code> |  |
| <code>const en() -&gt; Self</code> |  |
| <code>format_date(&amp;self, year: i32, month: u8, day: u8) -&gt; String</code> |  |
| <code>format_number(&amp;self, value: f64, fraction_digits: usize) -&gt; String</code> |  |
| <code>localize_number(&amp;self, plain: &amp;str) -&gt; String</code> |  |
| <code>const new(decimal: char, group: Option&lt;char&gt;) -&gt; Self</code> |  |
| <code>normalize_number(&amp;self, input: &amp;str) -&gt; Option&lt;String&gt;</code> |  |
| <code>parse_date(&amp;self, input: &amp;str) -&gt; Option&lt;(i32, u8, u8)&gt;</code> |  |
| <code>parse_number(&amp;self, input: &amp;str) -&gt; Option&lt;f64&gt;</code> |  |
| <code>const with_date(self, order: <a href="#dateorder">DateOrder</a>, separator: char) -&gt; Self</code> |  |
| <code>const with_group_size(self, size: u8) -&gt; Self</code> |  |
| <code>const with_zero(self, zero: char) -&gt; Self</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>date_order: <a href="#dateorder">DateOrder</a></code> |  |
| <code>date_separator: char</code> |  |
| <code>decimal: char</code> |  |
| <code>group: Option&lt;char&gt;</code> |  |
| <code>group_size: u8</code> |  |
| <code>zero: char</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Metrics`

**Methods**

| Method | Description |
| --- | --- |
| <code>control_height(self, size: <a href="#controlsize">ControlSize</a>) -&gt; f32</code> |  |
| <code>const control_width(self, size: <a href="#controlsize">ControlSize</a>) -&gt; f32</code> |  |
| <code>current() -&gt; Self</code> |  |
| <code>gap(self, size: <a href="#controlsize">ControlSize</a>) -&gt; f32</code> |  |
| <code>icon_size(self) -&gt; f32</code> |  |
| <code>const is_grid(self) -&gt; bool</code> |  |
| <code>overlay_padding(self) -&gt; f32</code> |  |
| <code>padding(self, size: <a href="#controlsize">ControlSize</a>) -&gt; f32</code> |  |
| <code>const radius(self, radius: <a href="#radius">Radius</a>) -&gt; <a href="#px">Px</a></code> |  |
| <code>const ring_offset(self) -&gt; f32</code> |  |
| <code>const ring_width(self) -&gt; f32</code> |  |
| <code>const window() -&gt; Self</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `MillisU16`

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Px`

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `RenderStyle`

**Fields**

| Field | Description |
| --- | --- |
| <code>background_color: Option&lt;<a href="#rgba8">Rgba8</a>&gt;</code> |  |
| <code>background_gradient: Option&lt;<a href="#bggradient">BgGradient</a>&gt;</code> |  |
| <code>border_bottom: Option&lt;<a href="#px">Px</a>&gt;</code> |  |
| <code>border_color: Option&lt;<a href="#rgba8">Rgba8</a>&gt;</code> |  |
| <code>border_left: Option&lt;<a href="#px">Px</a>&gt;</code> |  |
| <code>border_radius: Option&lt;<a href="#px">Px</a>&gt;</code> |  |
| <code>border_right: Option&lt;<a href="#px">Px</a>&gt;</code> |  |
| <code>border_top: Option&lt;<a href="#px">Px</a>&gt;</code> |  |
| <code>border_width: Option&lt;<a href="#px">Px</a>&gt;</code> |  |
| <code>opacity: u16</code> |  |
| <code>shadow: Option&lt;<a href="#elevation">Elevation</a>&gt;</code> |  |
| <code>text_color: Option&lt;<a href="#rgba8">Rgba8</a>&gt;</code> |  |
| <code>transform: <a href="#transform2d">Transform2D</a></code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `StateStyle`

**Methods**

| Method | Description |
| --- | --- |
| <code>const EMPTY: Self</code> |  |
| <code>const disabled(self) -&gt; &amp;'static [<a href="#styleatom">StyleAtom</a>]</code> |  |
| <code>const focused(self) -&gt; &amp;'static [<a href="#styleatom">StyleAtom</a>]</code> |  |
| <code>const hovered(self) -&gt; &amp;'static [<a href="#styleatom">StyleAtom</a>]</code> |  |
| <code>const new(rest: &amp;'static [<a href="#styleatom">StyleAtom</a>], focused: &amp;'static [<a href="#styleatom">StyleAtom</a>], hovered: &amp;'static [<a href="#styleatom">StyleAtom</a>], pressed: &amp;'static [<a href="#styleatom">StyleAtom</a>], disabled: &amp;'static [<a href="#styleatom">StyleAtom</a>]) -&gt; Self</code> |  |
| <code>const pressed(self) -&gt; &amp;'static [<a href="#styleatom">StyleAtom</a>]</code> |  |
| <code>const resolve(self, state: <a href="#stylestate">StyleState</a>) -&gt; &amp;'static [<a href="#styleatom">StyleAtom</a>]</code> |  |
| <code>const rest(self) -&gt; &amp;'static [<a href="#styleatom">StyleAtom</a>]</code> |  |
| <code>const with_disabled(self, atoms: &amp;'static [<a href="#styleatom">StyleAtom</a>]) -&gt; Self</code> |  |
| <code>const with_focused(self, atoms: &amp;'static [<a href="#styleatom">StyleAtom</a>]) -&gt; Self</code> |  |
| <code>const with_hovered(self, atoms: &amp;'static [<a href="#styleatom">StyleAtom</a>]) -&gt; Self</code> |  |
| <code>const with_pressed(self, atoms: &amp;'static [<a href="#styleatom">StyleAtom</a>]) -&gt; Self</code> |  |
| <code>const with_rest(self, atoms: &amp;'static [<a href="#styleatom">StyleAtom</a>]) -&gt; Self</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `StateStyleBuilder`


### `Strings`

**Methods**

| Method | Description |
| --- | --- |
| <code>english() -&gt; Self</code> |  |
| <code>fill(&amp;self, key: <a href="#stringkey">StringKey</a>, args: &amp;[&amp;dyn Display]) -&gt; String</code> |  |
| <code>from_table(table: &amp;[(<a href="#stringkey">StringKey</a>, &amp;'static str)]) -&gt; Self</code> |  |
| <code>get(&amp;self, key: <a href="#stringkey">StringKey</a>) -&gt; &amp;str</code> |  |
| <code>set(&amp;mut self, key: <a href="#stringkey">StringKey</a>, value: impl Into&lt;Cow&lt;'static, str&gt;&gt;)</code> |  |
| <code>with(self, key: <a href="#stringkey">StringKey</a>, value: impl Into&lt;Cow&lt;'static, str&gt;&gt;) -&gt; Self</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


### `StyleBinding`

**Methods**

| Method | Description |
| --- | --- |
| <code>animator_overlay(&amp;self) -&gt; Option&lt;&amp;dyn <a href="#animatoroverlay">AnimatorOverlay</a>&gt;</code> |  |
| <code>cascade(&amp;self) -&gt; impl Iterator&lt;Item = <a href="#styleatom">StyleAtom</a>&gt; + '_</code> |  |
| <code>dynamic_slice(&amp;self) -&gt; &amp;[<a href="#styleatom">StyleAtom</a>]</code> |  |
| <code>has_reactive(&amp;self) -&gt; bool</code> |  |
| <code>hides(&amp;self) -&gt; bool</code> |  |
| <code>interaction_slot(&amp;self) -&gt; <a href="#stateslot">StateSlot</a></code> |  |
| <code>owned_atoms(dynamic_atoms: SmallVec&lt;[<a href="#styleatom">StyleAtom</a>; 16]&gt;) -&gt; Self</code> |  |
| <code>reactive_atoms(reactive_atoms: <a href="#reactivestyleatoms">ReactiveStyleAtoms</a>) -&gt; Self</code> |  |
| <code>reactive_channels(&amp;self) -&gt; impl Iterator&lt;Item = &amp;'static [<a href="#styleatom">StyleAtom</a>]&gt; + '_</code> |  |
| <code>reactive_contains(&amp;self, target: &amp;<a href="#styleatom">StyleAtom</a>) -&gt; bool</code> |  |
| <code>retarget_animators(&amp;mut self, slot: <a href="#stateslot">StateSlot</a>, base: &amp;[<a href="#styleatom">StyleAtom</a>])</code> |  |
| <code>set_animator_overlay(&amp;mut self, overlay: Option&lt;Box&lt;dyn <a href="#animatoroverlay">AnimatorOverlay</a>&gt;&gt;)</code> |  |
| <code>set_state_overlay(&amp;mut self, overlay: <a href="#styleoverlay">StyleOverlay</a>, slot: <a href="#reactivestateslot">ReactiveStateSlot</a>)</code> |  |
| <code>state_overlay(&amp;self) -&gt; Option&lt;&amp;<a href="#styleoverlay">StyleOverlay</a>&gt;</code> |  |
| <code>state_overlay_atoms(&amp;self) -&gt; &amp;[<a href="#styleatom">StyleAtom</a>]</code> |  |
| <code>static_atoms(static_atoms: &amp;'static [<a href="#styleatom">StyleAtom</a>]) -&gt; Self</code> |  |
| <code>const static_slice(&amp;self) -&gt; &amp;'static [<a href="#styleatom">StyleAtom</a>]</code> |  |

**Trait implementations:** <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code><a href="#animatoroverlay">AnimatorOverlay</a></code>


### `StyleEngine`

**Methods**

| Method | Description |
| --- | --- |
| <code>pub fn <a href="#styleengine">StyleEngine</a>&lt;'theme&gt;::apply(&amp;self, atoms: &amp;[<a href="#styleatom">StyleAtom</a>], render: &amp;mut <a href="#renderstyle">RenderStyle</a>)</code> |  |
| <code>pub fn <a href="#styleengine">StyleEngine</a>&lt;'theme&gt;::apply_text(&amp;self, atoms: &amp;[<a href="#styleatom">StyleAtom</a>], text: &amp;mut <a href="#textstyle">TextStyle</a>)</code> |  |
| <code>pub const fn <a href="#styleengine">StyleEngine</a>&lt;'theme&gt;::new(theme: &amp;'theme <a href="#theme">Theme</a>) -&gt; Self</code> |  |
| <code>pub fn <a href="#styleengine">StyleEngine</a>&lt;'theme&gt;::resolve_color(&amp;self, color: <a href="#coloratom">ColorAtom</a>) -&gt; <a href="#rgba8">Rgba8</a></code> |  |
| <code>pub const fn <a href="#styleengine">StyleEngine</a>&lt;'theme&gt;::with_tone(theme: &amp;'theme <a href="#theme">Theme</a>, tone: <a href="#tone">Tone</a>) -&gt; Self</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Debug</code>, <code>Copy</code>


### `StyleOverlay`

**Methods**

| Method | Description |
| --- | --- |
| <code>atoms_for(&amp;self, slot: <a href="#stateslot">StateSlot</a>) -&gt; &amp;[<a href="#styleatom">StyleAtom</a>]</code> |  |
| <code>disabled(&amp;self) -&gt; &amp;[<a href="#styleatom">StyleAtom</a>]</code> |  |
| <code>focus(&amp;self) -&gt; &amp;[<a href="#styleatom">StyleAtom</a>]</code> |  |
| <code>const from_slots(rest: SmallVec&lt;[<a href="#styleatom">StyleAtom</a>; 4]&gt;, hover: SmallVec&lt;[<a href="#styleatom">StyleAtom</a>; 4]&gt;, focus: SmallVec&lt;[<a href="#styleatom">StyleAtom</a>; 4]&gt;, pressed: SmallVec&lt;[<a href="#styleatom">StyleAtom</a>; 4]&gt;, disabled: SmallVec&lt;[<a href="#styleatom">StyleAtom</a>; 4]&gt;) -&gt; Self</code> |  |
| <code>hover(&amp;self) -&gt; &amp;[<a href="#styleatom">StyleAtom</a>]</code> |  |
| <code>is_empty(&amp;self) -&gt; bool</code> |  |
| <code>new() -&gt; Self</code> |  |
| <code>pressed(&amp;self) -&gt; &amp;[<a href="#styleatom">StyleAtom</a>]</code> |  |
| <code>rest(&amp;self) -&gt; &amp;[<a href="#styleatom">StyleAtom</a>]</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


### `StyleReactiveEffect`

**Methods**

| Method | Description |
| --- | --- |
| <code>new() -&gt; Self</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Default</code>, <code>Debug</code>


### `StyleState`

**Methods**

| Method | Description |
| --- | --- |
| <code>const slot(self) -&gt; <a href="#stateslot">StateSlot</a></code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>disabled: bool</code> |  |
| <code>focused: bool</code> |  |
| <code>hovered: bool</code> |  |
| <code>pressed: bool</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `StyleVariant`

**Methods**

| Method | Description |
| --- | --- |
| <code>const base(self) -&gt; &amp;'static [<a href="#styleatom">StyleAtom</a>]</code> |  |
| <code>const new(base: &amp;'static [<a href="#styleatom">StyleAtom</a>], states: <a href="#statestyle">StateStyle</a>) -&gt; Self</code> |  |
| <code>const state(self, state: <a href="#stylestate">StyleState</a>) -&gt; &amp;'static [<a href="#styleatom">StyleAtom</a>]</code> |  |
| <code>const states(self) -&gt; <a href="#statestyle">StateStyle</a></code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `TextFace`

**Fields**

| Field | Description |
| --- | --- |
| <code>background: Option&lt;<a href="#token">Token</a>&gt;</code> |  |
| <code>bold: bool</code> |  |
| <code>color: Option&lt;<a href="#token">Token</a>&gt;</code> |  |
| <code>italic: bool</code> |  |
| <code>link: Option&lt;Arc&lt;str&gt;&gt;</code> |  |
| <code>mono: bool</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Hash</code>, <code>StructuralPartialEq</code>


### `TextRun`

**Fields**

| Field | Description |
| --- | --- |
| <code>end: usize</code> |  |
| <code>face: <a href="#textface">TextFace</a></code> |  |
| <code>start: usize</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>StructuralPartialEq</code>


### `TextRuns`

**Methods**

| Method | Description |
| --- | --- |
| <code>at(&amp;self, byte: usize) -&gt; Option&lt;&amp;<a href="#textrun">TextRun</a>&gt;</code> |  |
| <code>const family&lt;'a&gt;(face: &amp;<a href="#textface">TextFace</a>, base: &amp;'a str) -&gt; &amp;'a str</code> |  |
| <code>const weight(face: &amp;<a href="#textface">TextFace</a>, base: u16) -&gt; u16</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>StructuralPartialEq</code>


### `TextStyle`

**Methods**

| Method | Description |
| --- | --- |
| <code>layered_over_binding(base: &amp;Self, binding: &amp;<a href="#stylebinding">StyleBinding</a>) -&gt; Option&lt;Self&gt;</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>decoration: <a href="#textdecoration">TextDecoration</a></code> |  |
| <code>font_family: String</code> |  |
| <code>font_size: f32</code> |  |
| <code>font_weight: u16</code> |  |
| <code>italic: bool</code> |  |
| <code>letter_spacing_em: f32</code> |  |
| <code>line_height: <a href="#textlineheight">TextLineHeight</a></code> |  |
| <code>runs: Option&lt;<a href="#textruns">TextRuns</a>&gt;</code> |  |
| <code>tabular_nums: bool</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


### `ThemeBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>build(self) -&gt; <a href="#theme">Theme</a></code> |  |
| <code>const direction(self, direction: <a href="#direction">Direction</a>) -&gt; Self</code> |  |
| <code>const locale(self, locale: <a href="#locale">Locale</a>) -&gt; Self</code> |  |
| <code>new() -&gt; Self</code> |  |
| <code>strings(self, strings: <a href="#strings">Strings</a>) -&gt; Self</code> |  |
| <code>token(self, name: &amp;str, value: <a href="#themevalue">ThemeValue</a>) -&gt; Self</code> |  |
| <code>tokens(self, tokens: &amp;Map&lt;&amp;'static str, <a href="#themevalue">ThemeValue</a>&gt;) -&gt; Self</code> |  |

**Trait implementations:** <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


### `TokenLayer`

**Methods**

| Method | Description |
| --- | --- |
| <code>color(&amp;self, key: <a href="#colorkey">ColorKey</a>) -&gt; Option&lt;<a href="#rgba8">Rgba8</a>&gt;</code> |  |
| <code>insert(&amp;mut self, name: &amp;str, value: <a href="#themevalue">ThemeValue</a>) -&gt; Option&lt;<a href="#themevalue">ThemeValue</a>&gt;</code> |  |
| <code>names(&amp;self) -&gt; impl Iterator&lt;Item = &amp;str&gt;</code> |  |
| <code>value(&amp;self, name: &amp;str) -&gt; Option&lt;<a href="#themevalue">ThemeValue</a>&gt;</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


### `TokenTable`

**Fields**

| Field | Description |
| --- | --- |
| <code>overrides: <a href="#tokenlayer">TokenLayer</a></code> |  |
| <code>theme: <a href="#tokenlayer">TokenLayer</a></code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


### `Transform2D`

**Methods**

| Method | Description |
| --- | --- |
| <code>const IDENTITY: Self</code> |  |
| <code>origin(self, x: f32, y: f32) -&gt; Self</code> |  |
| <code>rotate(self, radians: f32) -&gt; Self</code> |  |
| <code>scale(self, x: f32, y: f32) -&gt; Self</code> |  |
| <code>translate(self, x: f32, y: f32) -&gt; Self</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `TransitionId`

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


## Enums

### `ColorAtom`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Rgba8(<a href="#rgba8">Rgba8</a>)</code> |  |
| <code>ThemeRef(<a href="#colorkey">ColorKey</a>)</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `ColorOrToken`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Atom(<a href="#coloratom">ColorAtom</a>)</code> |  |
| <code>Rgba(<a href="#rgba8">Rgba8</a>)</code> |  |
| <code>Token(<a href="#colorkey">ColorKey</a>)</code> |  |

**Trait implementations:** <code>From&lt;<a href="#coloratom">ColorAtom</a>&gt;</code>, <code>From&lt;<a href="#token">Token</a>&gt;</code>, <code>From&lt;<a href="#rgba8">Rgba8</a>&gt;</code>, <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `ControlSize`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Compact</code> |  |
| <code>Default</code> |  |
| <code>Grow</code> |  |
| <code>Wide</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Elevation`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Base</code> |  |
| <code>Lg</code> |  |
| <code>Md</code> |  |
| <code>Sm</code> |  |
| <code>Xl</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `FontFamily`

**Methods**

| Method | Description |
| --- | --- |
| <code>const name(self) -&gt; &amp;'static str</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>Display</code> |  |
| <code>Icon</code> |  |
| <code>Mono</code> |  |
| <code>Sans</code> |  |
| <code>Serif</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Radius`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Base</code> |  |
| <code>Full</code> |  |
| <code>Lg</code> |  |
| <code>Md</code> |  |
| <code>None</code> |  |
| <code>Sm</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Token`

**Methods**

| Method | Description |
| --- | --- |
| <code>const ALL: &amp;'static [Self]</code> |  |
| <code>const name(self) -&gt; &amp;'static str</code> |  |
| <code>const COUNT: usize</code> |  |
| <code>const color(self) -&gt; <a href="#coloratom">ColorAtom</a></code> |  |
| <code>const index(self) -&gt; usize</code> |  |
| <code>scale(role: <a href="sigil-style-scale.html#scalerole">ScaleRole</a>, step: u8) -&gt; Option&lt;Self&gt;</code> |  |
| <code>scale_step(self) -&gt; Option&lt;(<a href="sigil-style-scale.html#scalerole">ScaleRole</a>, u8)&gt;</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>Accent</code> |  |
| <code>Accent1</code> |  |
| <code>Accent10</code> |  |
| <code>Accent11</code> |  |
| <code>Accent12</code> |  |
| <code>Accent2</code> |  |
| <code>Accent3</code> |  |
| <code>Accent4</code> |  |
| <code>Accent5</code> |  |
| <code>Accent6</code> |  |
| <code>Accent7</code> |  |
| <code>Accent8</code> |  |
| <code>Accent9</code> |  |
| <code>AccentHover</code> |  |
| <code>Border</code> |  |
| <code>BorderStrong</code> |  |
| <code>BorderSubtle</code> |  |
| <code>Error1</code> |  |
| <code>Error10</code> |  |
| <code>Error11</code> |  |
| <code>Error12</code> |  |
| <code>Error2</code> |  |
| <code>Error3</code> |  |
| <code>Error4</code> |  |
| <code>Error5</code> |  |
| <code>Error6</code> |  |
| <code>Error7</code> |  |
| <code>Error8</code> |  |
| <code>Error9</code> |  |
| <code>Field</code> |  |
| <code>Gray1</code> |  |
| <code>Gray10</code> |  |
| <code>Gray11</code> |  |
| <code>Gray12</code> |  |
| <code>Gray2</code> |  |
| <code>Gray3</code> |  |
| <code>Gray4</code> |  |
| <code>Gray5</code> |  |
| <code>Gray6</code> |  |
| <code>Gray7</code> |  |
| <code>Gray8</code> |  |
| <code>Gray9</code> |  |
| <code>Info1</code> |  |
| <code>Info10</code> |  |
| <code>Info11</code> |  |
| <code>Info12</code> |  |
| <code>Info2</code> |  |
| <code>Info3</code> |  |
| <code>Info4</code> |  |
| <code>Info5</code> |  |
| <code>Info6</code> |  |
| <code>Info7</code> |  |
| <code>Info8</code> |  |
| <code>Info9</code> |  |
| <code>OnAccent</code> |  |
| <code>OnSolid</code> |  |
| <code>OnSurface</code> |  |
| <code>OnSurfaceMuted</code> |  |
| <code>Overlay</code> |  |
| <code>OverlayBorder</code> |  |
| <code>Ring</code> |  |
| <code>Scrim</code> |  |
| <code>Success1</code> |  |
| <code>Success10</code> |  |
| <code>Success11</code> |  |
| <code>Success12</code> |  |
| <code>Success2</code> |  |
| <code>Success3</code> |  |
| <code>Success4</code> |  |
| <code>Success5</code> |  |
| <code>Success6</code> |  |
| <code>Success7</code> |  |
| <code>Success8</code> |  |
| <code>Success9</code> |  |
| <code>Surface</code> |  |
| <code>SurfaceHover</code> |  |
| <code>SurfaceInset</code> |  |
| <code>SurfacePressed</code> |  |
| <code>SurfacePressedStrong</code> |  |
| <code>SurfaceSubtle</code> |  |
| <code>TableHeader</code> |  |
| <code>Warn1</code> |  |
| <code>Warn10</code> |  |
| <code>Warn11</code> |  |
| <code>Warn12</code> |  |
| <code>Warn2</code> |  |
| <code>Warn3</code> |  |
| <code>Warn4</code> |  |
| <code>Warn5</code> |  |
| <code>Warn6</code> |  |
| <code>Warn7</code> |  |
| <code>Warn8</code> |  |
| <code>Warn9</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `ColorKey`

**Methods**

| Method | Description |
| --- | --- |
| <code>const name(self) -&gt; &amp;'static str</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>Custom(&amp;'static str)</code> |  |
| <code>Token(<a href="#token">Token</a>)</code> |  |

**Trait implementations:** <code>From&lt;<a href="#token">Token</a>&gt;</code>, <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Tone`

**Methods**

| Method | Description |
| --- | --- |
| <code>fill(self) -&gt; <a href="#token">Token</a></code> |  |
| <code>const scale_role(self) -&gt; <a href="sigil-style-scale.html#scalerole">ScaleRole</a></code> |  |
| <code>shade(self) -&gt; <a href="#token">Token</a></code> |  |
| <code>solid(self) -&gt; <a href="#token">Token</a></code> |  |
| <code>token(self, step: u8) -&gt; <a href="#token">Token</a></code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>Error</code> |  |
| <code>Info</code> |  |
| <code>Neutral</code> |  |
| <code>Primary</code> |  |
| <code>Success</code> |  |
| <code>Warn</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>, <code><a href="sigil-reactive.html#reactive">Reactive</a>&lt;<a href="#tone">Tone</a>&gt;</code>


### `AlignKind`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Around</code> |  |
| <code>Between</code> |  |
| <code>Center</code> |  |
| <code>End</code> |  |
| <code>Start</code> |  |
| <code>Stretch</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `ColorAtomKind`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Background</code> |  |
| <code>Border</code> |  |
| <code>Text</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `DateOrder`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Dmy</code> |  |
| <code>Mdy</code> |  |
| <code>Ymd</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Dimension`

**Methods**

| Method | Description |
| --- | --- |
| <code>const px_or_zero(self) -&gt; f32</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>Auto</code> |  |
| <code>Fraction(u16)</code> |  |
| <code>Full</code> |  |
| <code>Px(<a href="#px">Px</a>)</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Direction`

**Methods**

| Method | Description |
| --- | --- |
| <code>const is_rtl(self) -&gt; bool</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>Ltr</code> |  |
| <code>Rtl</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `DisplayKind`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Block</code> |  |
| <code>Flex</code> |  |
| <code>Grid</code> |  |
| <code>None</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `FlexDir`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Column</code> |  |
| <code>ColumnReverse</code> |  |
| <code>Row</code> |  |
| <code>RowReverse</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `FlexWrapKind`

**Variants**

| Variant | Description |
| --- | --- |
| <code>NoWrap</code> |  |
| <code>Wrap</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `GradientAxis`

**Variants**

| Variant | Description |
| --- | --- |
| <code>BottomToTop</code> |  |
| <code>LeftToRight</code> |  |
| <code>RightToLeft</code> |  |
| <code>TopToBottom</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `JustifyKind`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Around</code> |  |
| <code>Between</code> |  |
| <code>Center</code> |  |
| <code>End</code> |  |
| <code>Evenly</code> |  |
| <code>Start</code> |  |
| <code>Stretch</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `LengthPercentage`

**Methods**

| Method | Description |
| --- | --- |
| <code>const px_or_zero(self) -&gt; f32</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>Percent(f32)</code> |  |
| <code>Px(<a href="#px">Px</a>)</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>From&lt;<a href="#px">Px</a>&gt;</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `NumericAtomKind`

**Variants**

| Variant | Description |
| --- | --- |
| <code>BorderRadius</code> |  |
| <code>FontSize</code> |  |
| <code>Gap</code> |  |
| <code>Height</code> |  |
| <code>LineHeight</code> |  |
| <code>Opacity</code> |  |
| <code>PaddingX</code> |  |
| <code>PaddingY</code> |  |
| <code>Rotation</code> |  |
| <code>ScaleX</code> |  |
| <code>ScaleY</code> |  |
| <code>TranslateX</code> |  |
| <code>TranslateY</code> |  |
| <code>Width</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `OverflowKind`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Auto</code> |  |
| <code>Hidden</code> |  |
| <code>Scroll</code> |  |
| <code>Visible</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `StateSlot`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Disabled</code> |  |
| <code>Focused</code> |  |
| <code>Hovered</code> |  |
| <code>Pressed</code> |  |
| <code>Rest</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `StringKey`

**Methods**

| Method | Description |
| --- | --- |
| <code>const ALL: [Self; 36]</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>Autosize</code> |  |
| <code>Avatar</code> |  |
| <code>Cancel</code> |  |
| <code>Chart</code> |  |
| <code>ChartSummary</code> |  |
| <code>Close</code> |  |
| <code>CodeEditor</code> |  |
| <code>CommandMenu</code> |  |
| <code>ContextMenu</code> |  |
| <code>Dialog</code> |  |
| <code>Empty</code> |  |
| <code>Heatmap</code> |  |
| <code>HeatmapGrid</code> |  |
| <code>HideColumn</code> |  |
| <code>KeyboardShortcuts</code> |  |
| <code>Loading</code> |  |
| <code>MenuBar</code> |  |
| <code>MoveLeft</code> |  |
| <code>MoveRight</code> |  |
| <code>NoData</code> |  |
| <code>Ok</code> |  |
| <code>RequiredMark</code> |  |
| <code>ResizePanes</code> |  |
| <code>ResizeSidebar</code> |  |
| <code>Search</code> |  |
| <code>Select</code> |  |
| <code>SelectAllRows</code> |  |
| <code>SelectRow</code> |  |
| <code>Series</code> |  |
| <code>ShowAllColumns</code> |  |
| <code>SortAscending</code> |  |
| <code>SortDescending</code> |  |
| <code>Sparkline</code> |  |
| <code>SparklineSummary</code> |  |
| <code>StatusBar</code> |  |
| <code>TableMenu</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `StyleAtom`

**Methods**

| Method | Description |
| --- | --- |
| <code>const affects_layout(self) -&gt; bool</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>AlignItems(<a href="#alignkind">AlignKind</a>)</code> |  |
| <code>BackgroundColor(<a href="#coloratom">ColorAtom</a>)</code> |  |
| <code>BackgroundGradient(<a href="#bggradient">BgGradient</a>)</code> |  |
| <code>BackgroundThemeRef(<a href="#colorkey">ColorKey</a>)</code> |  |
| <code>BgToneStep(u8)</code> |  |
| <code>BorderBottom(<a href="#px">Px</a>)</code> |  |
| <code>BorderColor(<a href="#coloratom">ColorAtom</a>)</code> |  |
| <code>BorderLeft(<a href="#px">Px</a>)</code> |  |
| <code>BorderRadius(<a href="#px">Px</a>)</code> |  |
| <code>BorderRight(<a href="#px">Px</a>)</code> |  |
| <code>BorderThemeRef(<a href="#colorkey">ColorKey</a>)</code> |  |
| <code>BorderToneStep(u8)</code> |  |
| <code>BorderTop(<a href="#px">Px</a>)</code> |  |
| <code>BorderWidth(<a href="#px">Px</a>)</code> |  |
| <code>Display(<a href="#displaykind">DisplayKind</a>)</code> |  |
| <code>FgOnSolid</code> |  |
| <code>FgToneStep(u8)</code> |  |
| <code>FlexDirection(<a href="#flexdir">FlexDir</a>)</code> |  |
| <code>FlexGrow(u16)</code> |  |
| <code>FlexShrink(u16)</code> |  |
| <code>FlexWrap(<a href="#flexwrapkind">FlexWrapKind</a>)</code> |  |
| <code>FontFamily(&amp;'static str)</code> |  |
| <code>FontItalic(bool)</code> |  |
| <code>FontSize(<a href="#px">Px</a>)</code> |  |
| <code>FontWeight(u16)</code> |  |
| <code>Gap(<a href="#px">Px</a>)</code> |  |
| <code>GapX(<a href="#px">Px</a>)</code> |  |
| <code>GapY(<a href="#px">Px</a>)</code> |  |
| <code>GridColumns(u16)</code> |  |
| <code>GridRows(u16)</code> |  |
| <code>Height(<a href="#dimension">Dimension</a>)</code> |  |
| <code>JustifyContent(<a href="#justifykind">JustifyKind</a>)</code> |  |
| <code>LetterSpacing(i16)</code> |  |
| <code>LineHeight(<a href="#px">Px</a>)</code> |  |
| <code>MarginAll(<a href="#px">Px</a>)</code> |  |
| <code>MarginBottom(<a href="#px">Px</a>)</code> |  |
| <code>MarginLeft(<a href="#px">Px</a>)</code> |  |
| <code>MarginRight(<a href="#px">Px</a>)</code> |  |
| <code>MarginTop(<a href="#px">Px</a>)</code> |  |
| <code>MarginX(<a href="#px">Px</a>)</code> |  |
| <code>MarginY(<a href="#px">Px</a>)</code> |  |
| <code>MaxHeight(<a href="#dimension">Dimension</a>)</code> |  |
| <code>MaxWidth(<a href="#dimension">Dimension</a>)</code> |  |
| <code>MinHeight(<a href="#dimension">Dimension</a>)</code> |  |
| <code>MinWidth(<a href="#dimension">Dimension</a>)</code> |  |
| <code>Opacity(u16)</code> |  |
| <code>OverflowX(<a href="#overflowkind">OverflowKind</a>)</code> |  |
| <code>OverflowY(<a href="#overflowkind">OverflowKind</a>)</code> |  |
| <code>PaddingAll(<a href="#px">Px</a>)</code> |  |
| <code>PaddingBottom(<a href="#px">Px</a>)</code> |  |
| <code>PaddingLeft(<a href="#px">Px</a>)</code> |  |
| <code>PaddingRight(<a href="#px">Px</a>)</code> |  |
| <code>PaddingTop(<a href="#px">Px</a>)</code> |  |
| <code>PaddingX(<a href="#px">Px</a>)</code> |  |
| <code>PaddingY(<a href="#px">Px</a>)</code> |  |
| <code>Radius(<a href="#radius">Radius</a>)</code> |  |
| <code>Shadow(<a href="#elevation">Elevation</a>)</code> |  |
| <code>TabularNums(bool)</code> |  |
| <code>TextColor(<a href="#coloratom">ColorAtom</a>)</code> |  |
| <code>TextDecoration(<a href="#textdecoration">TextDecoration</a>)</code> |  |
| <code>TextThemeRef(<a href="#colorkey">ColorKey</a>)</code> |  |
| <code>Transform(<a href="#transform2d">Transform2D</a>)</code> |  |
| <code>Transition(<a href="#transitionid">TransitionId</a>)</code> |  |
| <code>TransitionDuration(<a href="#millisu16">MillisU16</a>)</code> |  |
| <code>UnknownThemeRef(&amp;'static str)</code> |  |
| <code>Width(<a href="#dimension">Dimension</a>)</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `TextDecoration`

**Variants**

| Variant | Description |
| --- | --- |
| <code>LineThrough</code> |  |
| <code>None</code> |  |
| <code>Underline</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `TextLineHeight`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Absolute(f32)</code> |  |
| <code>FontSizeRelative(f32)</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `ThemeValue`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Color(<a href="#rgba8">Rgba8</a>)</code> |  |
| <code>Ms(u16)</code> |  |
| <code>Px(f32)</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


## Traits

### `AnimatorOverlay`

**Methods**

| Method | Description |
| --- | --- |
| <code>cache_key(&amp;self) -&gt; u64</code> |  |
| <code>retarget_to_slot(&amp;mut self, slot: <a href="#stateslot">StateSlot</a>, base: &amp;[<a href="#styleatom">StyleAtom</a>])</code> |  |
| <code>update(&amp;self, _atoms: &amp;[<a href="#styleatom">StyleAtom</a>], _theme: &amp;<a href="#theme">Theme</a>, _tone: <a href="#tone">Tone</a>, _inherited_text: <a href="#rgba8">Rgba8</a>)</code> |  |
| <code>value_for(&amp;self, kind: <a href="#numericatomkind">NumericAtomKind</a>) -&gt; Option&lt;f32&gt;</code> |  |
| <code>value_for_color(&amp;self, kind: <a href="#coloratomkind">ColorAtomKind</a>) -&gt; Option&lt;<a href="#rgba8">Rgba8</a>&gt;</code> |  |
| <code>visit_layout_values(&amp;self, visit: &amp;mut dyn FnMut(<a href="#numericatomkind">NumericAtomKind</a>, f32))</code> |  |


### `NodeStyleExt`

**Methods**

| Method | Description |
| --- | --- |
| <code>animate(self, kind: <a href="#numericatomkind">NumericAtomKind</a>, value: <a href="sigil-reactive.html#signal">Signal</a>&lt;f32&gt;) -&gt; Self</code> |  |
| <code>focus_ring(self, ring: <a href="#focusring">FocusRing</a>) -&gt; Self</code> |  |
| <code>paint_when&lt;F&gt;(self, active: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;bool&gt;, paint: F) -&gt; Self where F: FnOnce(<a href="#statestylebuilder">StateStyleBuilder</a>) -&gt; <a href="#statestylebuilder">StateStyleBuilder</a></code> |  |
| <code>presence(self, shown: <a href="sigil-reactive.html#signal">Signal</a>&lt;bool&gt;, config: <a href="sigil-animation_runtime.html#animationconfig">AnimationConfig</a>) -&gt; Self</code> |  |
| <code>theme(self, theme: <a href="#theme">Theme</a>) -&gt; Self</code> |  |
| <code>tone(self, tone: <a href="#tone">Tone</a>) -&gt; Self</code> |  |
| <code>visible_when(self, shown: impl <a href="sigil-reactive.html#reactive">Reactive</a>&lt;bool&gt;) -&gt; Self</code> |  |


### `Paint`

**Methods**

| Method | Description |
| --- | --- |
| <code>background_gradient(self, gradient: <a href="#bggradient">BgGradient</a>) -&gt; Self</code> |  |
| <code>bg(self, color: impl Into&lt;<a href="#colorortoken">ColorOrToken</a>&gt;) -&gt; Self</code> |  |
| <code>border_bottom(self, px: i16) -&gt; Self</code> |  |
| <code>border_color(self, color: impl Into&lt;<a href="#colorortoken">ColorOrToken</a>&gt;) -&gt; Self</code> |  |
| <code>border_left(self, px: i16) -&gt; Self</code> |  |
| <code>border_right(self, px: i16) -&gt; Self</code> |  |
| <code>border_top(self, px: i16) -&gt; Self</code> |  |
| <code>border_w(self, px: i16) -&gt; Self</code> |  |
| <code>duration(self, duration: <a href="#millisu16">MillisU16</a>) -&gt; Self</code> |  |
| <code>font_family(self, family: <a href="#fontfamily">FontFamily</a>) -&gt; Self</code> |  |
| <code>font_italic(self, italic: bool) -&gt; Self</code> |  |
| <code>font_size(self, px: f32) -&gt; Self</code> |  |
| <code>font_weight(self, weight: u16) -&gt; Self</code> |  |
| <code>letter_spacing_em(self, em: f32) -&gt; Self</code> |  |
| <code>line_height_px(self, px: f32) -&gt; Self</code> |  |
| <code>line_through(self) -&gt; Self</code> |  |
| <code>opacity(self, value: f32) -&gt; Self</code> |  |
| <code>rounded(self, radius: <a href="#radius">Radius</a>) -&gt; Self</code> |  |
| <code>shadow(self, elevation: <a href="#elevation">Elevation</a>) -&gt; Self</code> |  |
| <code>tabular_nums(self, on: bool) -&gt; Self</code> |  |
| <code>text_color(self, color: impl Into&lt;<a href="#colorortoken">ColorOrToken</a>&gt;) -&gt; Self</code> |  |
| <code>transform(self, transform: <a href="#transform2d">Transform2D</a>) -&gt; Self</code> |  |
| <code>transition(self, id: <a href="#transitionid">TransitionId</a>) -&gt; Self</code> |  |
| <code>underline(self) -&gt; Self</code> |  |


### `StateVariants`

**Methods**

| Method | Description |
| --- | --- |
| <code>disabled_style&lt;F&gt;(self, configure: F) -&gt; Self where F: FnOnce(<a href="#statestylebuilder">StateStyleBuilder</a>) -&gt; <a href="#statestylebuilder">StateStyleBuilder</a></code> |  |
| <code>focus&lt;F&gt;(self, configure: F) -&gt; Self where F: FnOnce(<a href="#statestylebuilder">StateStyleBuilder</a>) -&gt; <a href="#statestylebuilder">StateStyleBuilder</a></code> |  |
| <code>hover&lt;F&gt;(self, configure: F) -&gt; Self where F: FnOnce(<a href="#statestylebuilder">StateStyleBuilder</a>) -&gt; <a href="#statestylebuilder">StateStyleBuilder</a></code> |  |
| <code>pressed&lt;F&gt;(self, configure: F) -&gt; Self where F: FnOnce(<a href="#statestylebuilder">StateStyleBuilder</a>) -&gt; <a href="#statestylebuilder">StateStyleBuilder</a></code> |  |
| <code>rest&lt;F&gt;(self, configure: F) -&gt; Self where F: FnOnce(<a href="#statestylebuilder">StateStyleBuilder</a>) -&gt; <a href="#statestylebuilder">StateStyleBuilder</a></code> |  |


### `StyledTextEngine`

**Methods**

| Method | Description |
| --- | --- |
| <code>font_generation(&amp;self) -&gt; u64</code> |  |
| <code>layout_paragraph_styled(&amp;self, text: &amp;str, max_width: Option&lt;f32&gt;, style: &amp;<a href="#textstyle">TextStyle</a>) -&gt; Result&lt;Self::Paragraph, Self::Error&gt;</code> |  |


## Functions

### `font_family_hint`

<pre>pub const fn <a href="#font_family_hint">font_family_hint</a>(variant: <a href="sigil-style-variant.html#variant">Variant</a>) -&gt; <a href="sigil-style-variant.html#fontfamilyhint">FontFamilyHint</a></pre>


### `grid`

<pre>pub fn <a href="#grid">grid</a>(cell: f32)</pre>


### `is_typography_atom`

<pre>pub const fn <a href="#is_typography_atom">is_typography_atom</a>(atom: <a href="#styleatom">StyleAtom</a>) -&gt; bool</pre>


### `render_style_for`

<pre>pub fn <a href="#render_style_for">render_style_for</a>(id: <a href="sigil-kernel.html#nodeid">NodeId</a>, node: &amp;<a href="sigil-kernel.html#node">Node</a>, inherited: &amp;<a href="#theme">Theme</a>, inherited_text: Option&lt;<a href="#rgba8">Rgba8</a>&gt;) -&gt; <a href="#renderstyle">RenderStyle</a></pre>


### `space`

<pre>pub const fn <a href="#space">space</a>(steps: u8) -&gt; f32</pre>


### `text_style_for`

<pre>pub fn <a href="#text_style_for">text_style_for</a>(node: &amp;<a href="sigil-kernel.html#node">Node</a>, inherited: &amp;<a href="#theme">Theme</a>, base: &amp;<a href="#textstyle">TextStyle</a>) -&gt; <a href="#textstyle">TextStyle</a></pre>


### `unknown_class_warning_count`

<pre>pub fn <a href="#unknown_class_warning_count">unknown_class_warning_count</a>() -&gt; usize</pre>


## Type aliases

### `ComponentId`

<pre>pub type <a href="#componentid">ComponentId</a> = u64</pre>


### `ReactiveStateSlot`

<pre>pub type <a href="#reactivestateslot">ReactiveStateSlot</a> = Rc&lt;Cell&lt;<a href="#stateslot">StateSlot</a>&gt;&gt;</pre>


### `ReactiveStyleAtoms`

<pre>pub type <a href="#reactivestyleatoms">ReactiveStyleAtoms</a> = Rc&lt;Cell&lt;&amp;'static [<a href="#styleatom">StyleAtom</a>]&gt;&gt;</pre>


## Constants

### `BORDER_WIDTH`

<pre>pub const <a href="#border_width">BORDER_WIDTH</a>: <a href="#px">Px</a></pre>


### `DISABLED_OPACITY`

<pre>pub const <a href="#disabled_opacity">DISABLED_OPACITY</a>: u16</pre>


### `GHOST_VARIANT`

<pre>pub const <a href="#ghost_variant">GHOST_VARIANT</a>: <a href="#stylevariant">StyleVariant</a></pre>


### `ICON_SIZE`

<pre>pub const <a href="#icon_size">ICON_SIZE</a>: <a href="#px">Px</a></pre>


### `OVERLAY_SHADOW`

<pre>pub const <a href="#overlay_shadow">OVERLAY_SHADOW</a>: <a href="#elevation">Elevation</a></pre>


### `SELECTION_FILL`

<pre>pub const <a href="#selection_fill">SELECTION_FILL</a>: <a href="#rgba8">Rgba8</a></pre>


### `SOFT_VARIANT`

<pre>pub const <a href="#soft_variant">SOFT_VARIANT</a>: <a href="#stylevariant">StyleVariant</a></pre>


### `SOLID_VARIANT`

<pre>pub const <a href="#solid_variant">SOLID_VARIANT</a>: <a href="#stylevariant">StyleVariant</a></pre>


### `SPACING_STEP`

<pre>pub const <a href="#spacing_step">SPACING_STEP</a>: f32</pre>


---

Generated by truesight from `sigil-ui` 0.1.0. See the [API overview](index.md).
