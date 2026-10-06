# Module `sigil::text`

| Item | Kind | Description |
| --- | --- | --- |
| [`Paragraph`](#paragraph) | struct |  |
| [`OwnedTextEngine`](#ownedtextengine) | struct |  |
| [`Font`](#font) | struct |  |
| [`TextConfig`](#textconfig) | struct |  |
| [`WebFont`](#webfont) | struct |  |
| [`TextError`](#texterror) | enum |  |
| [`WebFontScript`](#webfontscript) | enum |  |
| [`add_fallback_font`](#add_fallback_font) | fn |  |
| [`prefer_system_fonts`](#prefer_system_fonts) | fn |  |
| [`set_web_fallbacks`](#set_web_fallbacks) | fn |  |
| [`web_fallbacks`](#web_fallbacks) | fn |  |

## Structs

### `Paragraph`

**Methods**

| Method | Description |
| --- | --- |
| <code>caret_rect(&amp;self, pos: <a href="sigil-hal.html#hitposition">HitPosition</a>, caret_width: f32) -&gt; <a href="sigil-geometry.html#rect">Rect</a></code> |  |
| <code>for_each_font&lt;F&gt;(&amp;self, f: F) where F: FnMut(u64, &amp;[u8], u32)</code> |  |
| <code>hit_test(&amp;self, x: f32, y: f32) -&gt; <a href="sigil-hal.html#hitposition">HitPosition</a></code> |  |
| <code>selection_rects(&amp;self, anchor: <a href="sigil-hal.html#hitposition">HitPosition</a>, focus: <a href="sigil-hal.html#hitposition">HitPosition</a>) -&gt; Vec&lt;<a href="sigil-geometry.html#rect">Rect</a>&gt;</code> |  |
| <code>selection_rects_with(&amp;self, anchor: <a href="sigil-hal.html#hitposition">HitPosition</a>, focus: <a href="sigil-hal.html#hitposition">HitPosition</a>, f: impl FnMut(<a href="sigil-geometry.html#rect">Rect</a>))</code> |  |
| <code>rendered_glyphs(&amp;self) -&gt; Vec&lt;<a href="sigil-hal.html#renderedglyph">RenderedGlyph</a>&gt;</code> |  |
| <code>rendered_glyphs_in(&amp;self, start: usize, end: usize) -&gt; Vec&lt;<a href="sigil-hal.html#renderedglyph">RenderedGlyph</a>&gt;</code> |  |

**Trait implementations:** <code><a href="sigil-hal.html#paragraphmetrics">ParagraphMetrics</a></code>, <code>Clone</code>


### `OwnedTextEngine`

**Methods**

| Method | Description |
| --- | --- |
| <code>clear_cache(&amp;self)</code> |  |
| <code>debug_segments_for_tests(&amp;self, text: &amp;str) -&gt; Vec&lt;(u32, u32, u8, &amp;'static str)&gt;</code> |  |
| <code>font_generation(&amp;self) -&gt; u64</code> |  |
| <code>layout_paragraph_ellipsized(&amp;self, text: &amp;str, max_width: f32, style: &amp;<a href="sigil-style.html#textstyle">TextStyle</a>) -&gt; Result&lt;<a href="#paragraph">Paragraph</a>, <a href="#texterror">TextError</a>&gt;</code> |  |
| <code>layout_paragraph_styled(&amp;self, text: &amp;str, max_width: Option&lt;f32&gt;, style: &amp;<a href="sigil-style.html#textstyle">TextStyle</a>) -&gt; Result&lt;<a href="#paragraph">Paragraph</a>, <a href="#texterror">TextError</a>&gt;</code> |  |
| <code>new(config: <a href="#textconfig">TextConfig</a>) -&gt; Self</code> |  |
| <code>register_font(&amp;mut self, bytes: Vec&lt;u8&gt;, face_index: u32) -&gt; Result&lt;(), <a href="#texterror">TextError</a>&gt;</code> |  |
| <code>register_font_family(&amp;mut self, family: &amp;str, bytes: Vec&lt;u8&gt;, face_index: u32) -&gt; Result&lt;(), <a href="#texterror">TextError</a>&gt;</code> |  |
| <code>thread() -&gt; Rc&lt;Self&gt;</code> |  |

**Trait implementations:** <code><a href="sigil-hal.html#textengine">TextEngine</a></code>, <code><a href="sigil-style.html#styledtextengine">StyledTextEngine</a></code>


### `Font`

**Methods**

| Method | Description |
| --- | --- |
| <code>bytes_arc(&amp;self) -&gt; Arc&lt;Vec&lt;u8&gt;&gt;</code> |  |
| <code>const face_index(&amp;self) -&gt; u32</code> |  |
| <code>from_bytes(data: Vec&lt;u8&gt;, index: u32) -&gt; Result&lt;Self, text::shaper::ShaperError&gt;</code> |  |
| <code>const id(&amp;self) -&gt; u64</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Debug</code>


### `TextConfig`

**Methods**

| Method | Description |
| --- | --- |
| <code>text_style(&amp;self) -&gt; <a href="sigil-style.html#textstyle">TextStyle</a></code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>font_family: String</code> |  |
| <code>font_size: f32</code> |  |
| <code>font_weight: u16</code> |  |
| <code>line_height: f32</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Default</code>, <code>Debug</code>


### `WebFont`

**Methods**

| Method | Description |
| --- | --- |
| <code>new(script: <a href="#webfontscript">WebFontScript</a>, url: impl Into&lt;String&gt;, face_index: u32) -&gt; Self</code> |  |
| <code>noto_defaults() -&gt; Vec&lt;Self&gt;</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>face_index: u32</code> |  |
| <code>script: <a href="#webfontscript">WebFontScript</a></code> |  |
| <code>url: String</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


## Enums

### `TextError`

**Variants**

| Variant | Description |
| --- | --- |
| <code>InvalidFont</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Error</code>, <code>Debug</code>, <code>Display</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `WebFontScript`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Arabic</code> |  |
| <code>Cjk</code> |  |
| <code>Devanagari</code> |  |
| <code>Hebrew</code> |  |
| <code>Khmer</code> |  |
| <code>Lao</code> |  |
| <code>Thai</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


## Functions

### `add_fallback_font`

<pre>pub fn <a href="#add_fallback_font">add_fallback_font</a>(bytes: Vec&lt;u8&gt;, face_index: u32) -&gt; Result&lt;(), <a href="#texterror">TextError</a>&gt;</pre>


### `prefer_system_fonts`

<pre>pub fn <a href="#prefer_system_fonts">prefer_system_fonts</a>()</pre>


### `set_web_fallbacks`

<pre>pub fn <a href="#set_web_fallbacks">set_web_fallbacks</a>(fonts: impl IntoIterator&lt;Item = <a href="#webfont">WebFont</a>&gt;)</pre>


### `web_fallbacks`

<pre>pub fn <a href="#web_fallbacks">web_fallbacks</a>() -&gt; Vec&lt;<a href="#webfont">WebFont</a>&gt;</pre>


---

Generated by truesight from `sigil-ui` 0.1.0. See the [API overview](index.md).
