# Module `sigil::scene`

| Item | Kind | Description |
| --- | --- | --- |
| [`CellBorder`](#cellborder) | struct |  |
| [`CellRect`](#cellrect) | struct |  |
| [`CommandScene`](#commandscene) | struct |  |
| [`CornerRadii`](#cornerradii) | struct |  |
| [`DecorationGeometry`](#decorationgeometry) | struct |  |
| [`DrawDecoration`](#drawdecoration) | struct |  |
| [`DrawGlyphRun`](#drawglyphrun) | struct |  |
| [`DrawGradient`](#drawgradient) | struct |  |
| [`DrawImage`](#drawimage) | struct |  |
| [`DrawPattern`](#drawpattern) | struct |  |
| [`DrawRect`](#drawrect) | struct |  |
| [`DrawSprite`](#drawsprite) | struct |  |
| [`GlyphRun`](#glyphrun) | struct |  |
| [`GradientStop`](#gradientstop) | struct |  |
| [`LayerScope`](#layerscope) | struct |  |
| [`LinearGradient`](#lineargradient) | struct |  |
| [`SceneBuilder`](#scenebuilder) | struct |  |
| [`SceneDecorationStyle`](#scenedecorationstyle) | struct |  |
| [`SceneGradientStyle`](#scenegradientstyle) | struct |  |
| [`ScenePatternStyle`](#scenepatternstyle) | struct |  |
| [`SceneRectStyle`](#scenerectstyle) | struct |  |
| [`SceneSpriteStyle`](#scenespritestyle) | struct |  |
| [`Shadow`](#shadow) | struct |  |
| [`SpriteSource`](#spritesource) | struct |  |
| [`Stroke`](#stroke) | struct |  |
| [`DecorationKind`](#decorationkind) | enum |  |
| [`LayerEffect`](#layereffect) | enum |  |
| [`PaintFill`](#paintfill) | enum |  |
| [`PatternKind`](#patternkind) | enum |  |
| [`SceneCommand`](#scenecommand) | enum |  |
| [`ShadowKind`](#shadowkind) | enum |  |
| [`SpriteKind`](#spritekind) | enum |  |
| [`StrokeAlign`](#strokealign) | enum |  |
| [`StrokeStyle`](#strokestyle) | enum |  |
| [`TextFlow`](#textflow) | enum |  |
| [`emit_scene`](#emit_scene) | fn |  |
| [`emit_scene_clipped`](#emit_scene_clipped) | fn |  |
| [`shape_glyph_run`](#shape_glyph_run) | fn |  |
| [`Transform`](#transform) | type |  |

## Structs

### `CellBorder`

**Fields**

| Field | Description |
| --- | --- |
| <code>color: <a href="sigil-style.html#rgba8">Rgba8</a></code> |  |
| <code>rect: <a href="sigil-geometry.html#rect">Rect</a></code> |  |
| <code>rounded: bool</code> |  |
| <code>widths: [f32; 4]</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `CellRect`

**Fields**

| Field | Description |
| --- | --- |
| <code>border: Option&lt;<a href="#cellborder">CellBorder</a>&gt;</code> |  |
| <code>leaf: bool</code> |  |
| <code>outline: bool</code> |  |
| <code>tone: Option&lt;<a href="sigil-style.html#tone">Tone</a>&gt;</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `CommandScene`

**Fields**

| Field | Description |
| --- | --- |
| <code>commands: Vec&lt;<a href="#scenecommand">SceneCommand</a>&gt;</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


### `CornerRadii`

**Methods**

| Method | Description |
| --- | --- |
| <code>const ZERO: Self</code> |  |
| <code>const bottom(radius: f32) -&gt; Self</code> |  |
| <code>const left(radius: f32) -&gt; Self</code> |  |
| <code>const new(top_left: f32, top_right: f32, bottom_right: f32, bottom_left: f32) -&gt; Self</code> |  |
| <code>normalized_for(self, width: f32, height: f32) -&gt; [f32; 4]</code> |  |
| <code>const right(radius: f32) -&gt; Self</code> |  |
| <code>const top(radius: f32) -&gt; Self</code> |  |
| <code>const uniform(radius: f32) -&gt; Self</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `DecorationGeometry`

**Methods**

| Method | Description |
| --- | --- |
| <code>line(&amp;self) -&gt; <a href="sigil-geometry.html#rect">Rect</a></code> |  |
| <code>reach(&amp;self) -&gt; f32</code> |  |
| <code>wave_y(&amp;self, x: f32) -&gt; f32</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>amplitude: f32</code> |  |
| <code>band: <a href="sigil-geometry.html#rect">Rect</a></code> |  |
| <code>center: f32</code> |  |
| <code>period: f32</code> |  |
| <code>thickness: f32</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `DrawDecoration`

**Methods**

| Method | Description |
| --- | --- |
| <code>geometry(&amp;self) -&gt; Option&lt;<a href="#decorationgeometry">DecorationGeometry</a>&gt;</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>band: <a href="sigil-geometry.html#rect">Rect</a></code> |  |
| <code>color: <a href="sigil-style.html#rgba8">Rgba8</a></code> |  |
| <code>kind: <a href="#decorationkind">DecorationKind</a></code> |  |
| <code>line_width: f32</code> |  |
| <code>node: Option&lt;<a href="sigil-kernel.html#nodeid">NodeId</a>&gt;</code> |  |
| <code>opacity: u16</code> |  |
| <code>period: f32</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `DrawGlyphRun`

**Methods**

| Method | Description |
| --- | --- |
| <code>const new(run: <a href="#glyphrun">GlyphRun</a>) -&gt; Self</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>run: <a href="#glyphrun">GlyphRun</a></code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


### `DrawGradient`

**Methods**

| Method | Description |
| --- | --- |
| <code>new(rect: <a href="sigil-geometry.html#rect">Rect</a>, style: <a href="#scenegradientstyle">SceneGradientStyle</a>) -&gt; Self</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>border: Option&lt;<a href="#cellborder">CellBorder</a>&gt;</code> |  |
| <code>border_radius: <a href="#cornerradii">CornerRadii</a></code> |  |
| <code>gradient: <a href="#lineargradient">LinearGradient</a></code> |  |
| <code>leaf: bool</code> |  |
| <code>node: Option&lt;<a href="sigil-kernel.html#nodeid">NodeId</a>&gt;</code> |  |
| <code>opacity: u16</code> |  |
| <code>outline: bool</code> |  |
| <code>rect: <a href="sigil-geometry.html#rect">Rect</a></code> |  |
| <code>stroke: Option&lt;<a href="#stroke">Stroke</a>&gt;</code> |  |
| <code>tone: Option&lt;<a href="sigil-style.html#tone">Tone</a>&gt;</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


### `DrawImage`

**Methods**

| Method | Description |
| --- | --- |
| <code>source_at(&amp;self, scale: f32) -&gt; (<a href="sigil-kernel.html#imagesource">ImageSource</a>, <a href="sigil-geometry.html#rect">Rect</a>)</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>crop: Option&lt;<a href="sigil-kernel.html#imagecrop">ImageCrop</a>&gt;</code> |  |
| <code>destination: <a href="sigil-geometry.html#rect">Rect</a></code> |  |
| <code>fit: <a href="sigil-kernel.html#objectfit">ObjectFit</a></code> |  |
| <code>node: Option&lt;<a href="sigil-kernel.html#nodeid">NodeId</a>&gt;</code> |  |
| <code>opacity: u16</code> |  |
| <code>raster: Option&lt;<a href="sigil-kernel.html#imageraster">ImageRaster</a>&gt;</code> |  |
| <code>source: <a href="sigil-kernel.html#imagesource">ImageSource</a></code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


### `DrawPattern`

**Methods**

| Method | Description |
| --- | --- |
| <code>const new(rect: <a href="sigil-geometry.html#rect">Rect</a>, style: <a href="#scenepatternstyle">ScenePatternStyle</a>) -&gt; Self</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>border_radius: <a href="#cornerradii">CornerRadii</a></code> |  |
| <code>node: Option&lt;<a href="sigil-kernel.html#nodeid">NodeId</a>&gt;</code> |  |
| <code>opacity: u16</code> |  |
| <code>pattern: <a href="#patternkind">PatternKind</a></code> |  |
| <code>rect: <a href="sigil-geometry.html#rect">Rect</a></code> |  |
| <code>stroke: Option&lt;<a href="#stroke">Stroke</a>&gt;</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `DrawRect`

**Methods**

| Method | Description |
| --- | --- |
| <code>const new(rect: <a href="sigil-geometry.html#rect">Rect</a>, style: <a href="#scenerectstyle">SceneRectStyle</a>) -&gt; Self</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>border: Option&lt;<a href="#cellborder">CellBorder</a>&gt;</code> |  |
| <code>border_radius: <a href="#cornerradii">CornerRadii</a></code> |  |
| <code>color: <a href="sigil-style.html#rgba8">Rgba8</a></code> |  |
| <code>leaf: bool</code> |  |
| <code>node: Option&lt;<a href="sigil-kernel.html#nodeid">NodeId</a>&gt;</code> |  |
| <code>opacity: u16</code> |  |
| <code>outline: bool</code> |  |
| <code>rect: <a href="sigil-geometry.html#rect">Rect</a></code> |  |
| <code>stroke: Option&lt;<a href="#stroke">Stroke</a>&gt;</code> |  |
| <code>tone: Option&lt;<a href="sigil-style.html#tone">Tone</a>&gt;</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `DrawSprite`

**Methods**

| Method | Description |
| --- | --- |
| <code>const new(style: &amp;<a href="#scenespritestyle">SceneSpriteStyle</a>) -&gt; Self</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>color: <a href="sigil-style.html#rgba8">Rgba8</a></code> |  |
| <code>destination: <a href="sigil-geometry.html#rect">Rect</a></code> |  |
| <code>kind: <a href="#spritekind">SpriteKind</a></code> |  |
| <code>node: Option&lt;<a href="sigil-kernel.html#nodeid">NodeId</a>&gt;</code> |  |
| <code>opacity: u16</code> |  |
| <code>source: <a href="#spritesource">SpriteSource</a></code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


### `GlyphRun`

**Methods**

| Method | Description |
| --- | --- |
| <code>new(rect: <a href="sigil-geometry.html#rect">Rect</a>, text: impl Into&lt;String&gt;, color: <a href="sigil-style.html#rgba8">Rgba8</a>, style: <a href="sigil-style.html#textstyle">TextStyle</a>, opacity: u16) -&gt; Self</code> |  |
| <code>positioned_glyphs(&amp;self) -&gt; &amp;[<a href="sigil-hal.html#renderedglyph">RenderedGlyph</a>]</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>node: Option&lt;<a href="sigil-kernel.html#nodeid">NodeId</a>&gt;</code> |  |
| <code>rect: <a href="sigil-geometry.html#rect">Rect</a></code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Deref</code>


### `GradientStop`

**Methods**

| Method | Description |
| --- | --- |
| <code>const new(offset: f32, color: <a href="sigil-style.html#rgba8">Rgba8</a>) -&gt; Self</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>color: <a href="sigil-style.html#rgba8">Rgba8</a></code> |  |
| <code>offset: f32</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `LayerScope`

**Fields**

| Field | Description |
| --- | --- |
| <code>clip: <a href="sigil-geometry.html#rect">Rect</a></code> |  |
| <code>clip_radii: <a href="#cornerradii">CornerRadii</a></code> |  |
| <code>effect: <a href="#layereffect">LayerEffect</a></code> |  |
| <code>opacity: f32</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `LinearGradient`

**Methods**

| Method | Description |
| --- | --- |
| <code>new(start: <a href="sigil-geometry.html#point">Point</a>, end: <a href="sigil-geometry.html#point">Point</a>, stops: impl Into&lt;SmallVec&lt;[<a href="#gradientstop">GradientStop</a>; 4]&gt;&gt;) -&gt; Self</code> |  |
| <code>normalized(&amp;self) -&gt; Self</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>end: <a href="sigil-geometry.html#point">Point</a></code> |  |
| <code>start: <a href="sigil-geometry.html#point">Point</a></code> |  |
| <code>stops: SmallVec&lt;[<a href="#gradientstop">GradientStop</a>; 4]&gt;</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


### `SceneBuilder`

**Methods**

| Method | Description |
| --- | --- |
| <code>draw_decoration(&amp;mut self, style: <a href="#scenedecorationstyle">SceneDecorationStyle</a>)</code> |  |
| <code>draw_glyph_run(&amp;mut self, run: <a href="#glyphrun">GlyphRun</a>)</code> |  |
| <code>draw_glyph_run_for_node(&amp;mut self, node: <a href="sigil-kernel.html#nodeid">NodeId</a>, run: <a href="#glyphrun">GlyphRun</a>)</code> |  |
| <code>draw_gradient(&amp;mut self, rect: <a href="sigil-geometry.html#rect">Rect</a>, style: <a href="#scenegradientstyle">SceneGradientStyle</a>)</code> |  |
| <code>draw_image(&amp;mut self, draw: <a href="#drawimage">DrawImage</a>)</code> |  |
| <code>draw_pattern(&amp;mut self, rect: <a href="sigil-geometry.html#rect">Rect</a>, style: <a href="#scenepatternstyle">ScenePatternStyle</a>)</code> |  |
| <code>draw_rect(&amp;mut self, rect: <a href="sigil-geometry.html#rect">Rect</a>, style: <a href="#scenerectstyle">SceneRectStyle</a>)</code> |  |
| <code>draw_shadow(&amp;mut self, rect: <a href="sigil-geometry.html#rect">Rect</a>, radii: <a href="#cornerradii">CornerRadii</a>, shadow: <a href="#shadow">Shadow</a>)</code> |  |
| <code>draw_sprite(&amp;mut self, source: <a href="#spritesource">SpriteSource</a>, dest: <a href="sigil-geometry.html#rect">Rect</a>, kind: <a href="#spritekind">SpriteKind</a>, color: <a href="sigil-style.html#rgba8">Rgba8</a>) -&gt; &amp;mut Self</code> |  |
| <code>finish(self) -&gt; <a href="#commandscene">CommandScene</a></code> |  |
| <code>const new() -&gt; Self</code> |  |
| <code>pop_layer(&amp;mut self)</code> |  |
| <code>push_layer(&amp;mut self, scope: <a href="#layerscope">LayerScope</a>)</code> |  |
| <code>with_clip&lt;F: FnOnce(&amp;mut Self)&gt;(&amp;mut self, rect: <a href="sigil-geometry.html#rect">Rect</a>, body: F)</code> |  |
| <code>with_composite_layer&lt;F: FnOnce(&amp;mut Self)&gt;(&amp;mut self, scope: <a href="#layerscope">LayerScope</a>, body: F)</code> |  |
| <code>with_layer&lt;F: FnOnce(&amp;mut Self)&gt;(&amp;mut self, opacity: f32, body: F)</code> |  |
| <code>with_transform&lt;F: FnOnce(&amp;mut Self)&gt;(&amp;mut self, transform: <a href="#transform">Transform</a>, body: F)</code> |  |

**Trait implementations:** <code>Default</code>, <code>Debug</code>


### `SceneDecorationStyle`

**Fields**

| Field | Description |
| --- | --- |
| <code>band: <a href="sigil-geometry.html#rect">Rect</a></code> |  |
| <code>color: <a href="sigil-style.html#rgba8">Rgba8</a></code> |  |
| <code>kind: <a href="#decorationkind">DecorationKind</a></code> |  |
| <code>line_width: f32</code> |  |
| <code>opacity: u16</code> |  |
| <code>period: f32</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `SceneGradientStyle`

**Methods**

| Method | Description |
| --- | --- |
| <code>const new(gradient: <a href="#lineargradient">LinearGradient</a>, border_radius: <a href="#cornerradii">CornerRadii</a>, opacity: u16) -&gt; Self</code> |  |
| <code>const with_stroke(self, stroke: <a href="#stroke">Stroke</a>) -&gt; Self</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>border_radius: <a href="#cornerradii">CornerRadii</a></code> |  |
| <code>gradient: <a href="#lineargradient">LinearGradient</a></code> |  |
| <code>opacity: u16</code> |  |
| <code>stroke: Option&lt;<a href="#stroke">Stroke</a>&gt;</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


### `ScenePatternStyle`

**Methods**

| Method | Description |
| --- | --- |
| <code>const new(pattern: <a href="#patternkind">PatternKind</a>, border_radius: <a href="#cornerradii">CornerRadii</a>, opacity: u16) -&gt; Self</code> |  |
| <code>const with_stroke(self, stroke: <a href="#stroke">Stroke</a>) -&gt; Self</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>border_radius: <a href="#cornerradii">CornerRadii</a></code> |  |
| <code>opacity: u16</code> |  |
| <code>pattern: <a href="#patternkind">PatternKind</a></code> |  |
| <code>stroke: Option&lt;<a href="#stroke">Stroke</a>&gt;</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `SceneRectStyle`

**Methods**

| Method | Description |
| --- | --- |
| <code>const new(color: <a href="sigil-style.html#rgba8">Rgba8</a>, border_radius: <a href="#cornerradii">CornerRadii</a>, opacity: u16) -&gt; Self</code> |  |
| <code>const rounded(color: <a href="sigil-style.html#rgba8">Rgba8</a>, radius: f32) -&gt; Self</code> |  |
| <code>const solid(color: <a href="sigil-style.html#rgba8">Rgba8</a>) -&gt; Self</code> |  |
| <code>const with_stroke(self, stroke: <a href="#stroke">Stroke</a>) -&gt; Self</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>border_radius: <a href="#cornerradii">CornerRadii</a></code> |  |
| <code>color: <a href="sigil-style.html#rgba8">Rgba8</a></code> |  |
| <code>opacity: u16</code> |  |
| <code>stroke: Option&lt;<a href="#stroke">Stroke</a>&gt;</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `SceneSpriteStyle`

**Fields**

| Field | Description |
| --- | --- |
| <code>color: <a href="sigil-style.html#rgba8">Rgba8</a></code> |  |
| <code>destination: <a href="sigil-geometry.html#rect">Rect</a></code> |  |
| <code>kind: <a href="#spritekind">SpriteKind</a></code> |  |
| <code>opacity: u16</code> |  |
| <code>source: <a href="#spritesource">SpriteSource</a></code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


### `Shadow`

**Methods**

| Method | Description |
| --- | --- |
| <code>glow(color: <a href="sigil-style.html#rgba8">Rgba8</a>, spread: f32, blur: f32) -&gt; Self</code> |  |
| <code>const new(offset: [f32; 2], blur: f32, spread: f32, color: [f32; 4]) -&gt; Self</code> |  |
| <code>const with_kind(self, kind: <a href="#shadowkind">ShadowKind</a>) -&gt; Self</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>blur: f32</code> |  |
| <code>color: [f32; 4]</code> |  |
| <code>kind: <a href="#shadowkind">ShadowKind</a></code> |  |
| <code>offset: [f32; 2]</code> |  |
| <code>spread: f32</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `SpriteSource`

**Fields**

| Field | Description |
| --- | --- |
| <code>atlas: u32</code> |  |
| <code>phase: f32</code> |  |
| <code>region: <a href="sigil-geometry.html#rect">Rect</a></code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Stroke`

**Fields**

| Field | Description |
| --- | --- |
| <code>align: <a href="#strokealign">StrokeAlign</a></code> |  |
| <code>color: <a href="sigil-style.html#rgba8">Rgba8</a></code> |  |
| <code>style: <a href="#strokestyle">StrokeStyle</a></code> |  |
| <code>width: f32</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


## Enums

### `DecorationKind`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Strikethrough</code> |  |
| <code>Underline</code> |  |
| <code>WavyUnderline</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `LayerEffect`

**Fields**

| Field | Description |
| --- | --- |
| <code>LayerBlur::sigma: f32</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>Identity</code> |  |
| <code>LayerBlur</code> |  |
| <code>Transform(<a href="#transform">Transform</a>)</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `PaintFill`

**Variants**

| Variant | Description |
| --- | --- |
| <code>LinearGradient(<a href="#lineargradient">LinearGradient</a>)</code> |  |
| <code>Pattern(<a href="#patternkind">PatternKind</a>)</code> |  |
| <code>Solid(<a href="sigil-style.html#rgba8">Rgba8</a>)</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


### `PatternKind`

**Methods**

| Method | Description |
| --- | --- |
| <code>const normalized(self) -&gt; Self</code> |  |
| <code>const tile_extent(self) -&gt; f32</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>Checker::cell: f32</code> |  |
| <code>Checker::color_a: <a href="sigil-style.html#rgba8">Rgba8</a></code> |  |
| <code>Checker::color_b: <a href="sigil-style.html#rgba8">Rgba8</a></code> |  |
| <code>Slash::color: <a href="sigil-style.html#rgba8">Rgba8</a></code> |  |
| <code>Slash::line_width: f32</code> |  |
| <code>Slash::period: f32</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>Checker</code> |  |
| <code>Slash</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `SceneCommand`

**Fields**

| Field | Description |
| --- | --- |
| <code>DrawShadow::radii: <a href="#cornerradii">CornerRadii</a></code> |  |
| <code>DrawShadow::rect: [f32; 4]</code> |  |
| <code>DrawShadow::shadow: <a href="#shadow">Shadow</a></code> |  |
| <code>SaveLayer::opacity: f32</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>Decoration(<a href="#drawdecoration">DrawDecoration</a>)</code> |  |
| <code>DrawShadow</code> |  |
| <code>GlyphRun(<a href="#drawglyphrun">DrawGlyphRun</a>)</code> |  |
| <code>Gradient(<a href="#drawgradient">DrawGradient</a>)</code> |  |
| <code>Image(<a href="#drawimage">DrawImage</a>)</code> |  |
| <code>Pattern(<a href="#drawpattern">DrawPattern</a>)</code> |  |
| <code>PopClip</code> |  |
| <code>PopLayer</code> |  |
| <code>PopTransform</code> |  |
| <code>PushClip(<a href="sigil-geometry.html#rect">Rect</a>)</code> |  |
| <code>PushLayer(<a href="#layerscope">LayerScope</a>)</code> |  |
| <code>PushTransform(<a href="#transform">Transform</a>)</code> |  |
| <code>Rect(<a href="#drawrect">DrawRect</a>)</code> |  |
| <code>RestoreLayer</code> |  |
| <code>SaveLayer</code> |  |
| <code>Sprite(<a href="#drawsprite">DrawSprite</a>)</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


### `ShadowKind`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Glow</code> |  |
| <code>Inner</code> |  |
| <code>Outer</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `SpriteKind`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Mono</code> |  |
| <code>Poly</code> |  |
| <code>Subpixel</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `StrokeAlign`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Centered</code> |  |
| <code>Inside</code> |  |
| <code>Outside</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `StrokeStyle`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Dashed(f32)</code> |  |
| <code>Solid</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `TextFlow`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Ellipsis</code> |  |
| <code>Line</code> |  |
| <code>Wrap</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


## Functions

### `emit_scene`

<pre>pub fn <a href="#emit_scene">emit_scene</a>(cx: &amp;<a href="sigil-kernel.html#cx">Cx</a>, root: <a href="sigil-kernel.html#nodeid">NodeId</a>) -&gt; <a href="#commandscene">CommandScene</a></pre>


### `emit_scene_clipped`

<pre>pub fn <a href="#emit_scene_clipped">emit_scene_clipped</a>(cx: &amp;<a href="sigil-kernel.html#cx">Cx</a>, root: <a href="sigil-kernel.html#nodeid">NodeId</a>, clip: Option&lt;<a href="sigil-geometry.html#rect">Rect</a>&gt;) -&gt; <a href="#commandscene">CommandScene</a></pre>


### `shape_glyph_run`

<pre>pub fn <a href="#shape_glyph_run">shape_glyph_run</a>(rect: <a href="sigil-geometry.html#rect">Rect</a>, text: String, color: <a href="sigil-style.html#rgba8">Rgba8</a>, style: <a href="sigil-style.html#textstyle">TextStyle</a>, opacity: u16) -&gt; <a href="#glyphrun">GlyphRun</a></pre>


## Type aliases

### `Transform`

<pre>pub type <a href="#transform">Transform</a> = [[f32; 3]; 2]</pre>


---

Generated by truesight from `sigil-ui` 0.1.0. See the [API overview](index.md).
