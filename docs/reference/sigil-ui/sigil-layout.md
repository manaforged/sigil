# Module `sigil::layout`

| Item | Kind | Description |
| --- | --- | --- |
| [`BoundsReactive`](#boundsreactive) | struct |  |
| [`CaretGeometry`](#caretgeometry) | struct |  |
| [`LayoutEngine`](#layoutengine) | struct |  |
| [`LayoutResult`](#layoutresult) | struct |  |
| [`LayoutStyle`](#layoutstyle) | struct |  |
| [`ReactivePosition`](#reactiveposition) | struct |  |
| [`Rect`](#rect) | struct |  |
| [`Size`](#size) | struct |  |
| [`StyleData`](#styledata) | struct |  |
| [`AlignContent`](#aligncontent) | enum |  |
| [`AlignItems`](#alignitems) | enum |  |
| [`BoxSizing`](#boxsizing) | enum |  |
| [`Dimension`](#dimension) | enum |  |
| [`Display`](#display) | enum |  |
| [`FlexDirection`](#flexdirection) | enum |  |
| [`FlexWrap`](#flexwrap) | enum |  |
| [`JustifyContent`](#justifycontent) | enum |  |
| [`LayoutError`](#layouterror) | enum |  |
| [`LengthPercentage`](#lengthpercentage) | enum |  |
| [`LengthPercentageAuto`](#lengthpercentageauto) | enum |  |
| [`Overflow`](#overflow) | enum |  |
| [`Position`](#position) | enum |  |
| [`TextLength`](#textlength) | enum |  |
| [`TrackSize`](#tracksize) | enum |  |
| [`IntoPx`](#intopx) | trait |  |
| [`LayoutBuilderExt`](#layoutbuilderext) | trait |  |
| [`LayoutReactiveExt`](#layoutreactiveext) | trait |  |
| [`TextGeometry`](#textgeometry) | trait |  |
| [`TextMeasure`](#textmeasure) | trait |  |
| [`cols`](#cols) | fn |  |
| [`column`](#column) | fn |  |
| [`em`](#em) | fn |  |
| [`row`](#row) | fn |  |
| [`rows`](#rows) | fn |  |
| [`AlignSelf`](#alignself) | type |  |

## Structs

### `BoundsReactive`

**Methods**

| Method | Description |
| --- | --- |
| <code>publish(&amp;self, bounds: (f32, f32, f32, f32)) -&gt; bool</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Copy</code>


### `CaretGeometry`

**Fields**

| Field | Description |
| --- | --- |
| <code>height: f32</code> |  |
| <code>top: f32</code> |  |
| <code>x: f32</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `LayoutEngine`

**Methods**

| Method | Description |
| --- | --- |
| <code>compute(&amp;mut self, cx: &amp;mut <a href="sigil-kernel.html#cx">Cx</a>, root: <a href="sigil-kernel.html#nodeid">NodeId</a>, available: <a href="sigil-geometry.html#size">Size</a>) -&gt; Result&lt;(), <a href="#layouterror">LayoutError</a>&gt;</code> |  |
| <code>gc(&amp;mut self, cx: &amp;<a href="sigil-kernel.html#cx">Cx</a>) -&gt; Result&lt;(), <a href="#layouterror">LayoutError</a>&gt;</code> |  |
| <code>const generation(&amp;self) -&gt; u64</code> |  |
| <code>mirrored_node_count(&amp;self) -&gt; usize</code> |  |
| <code>result(&amp;self, id: <a href="sigil-kernel.html#nodeid">NodeId</a>) -&gt; Option&lt;<a href="#layoutresult">LayoutResult</a>&gt;</code> |  |
| <code>sync(&amp;mut self, cx: &amp;<a href="sigil-kernel.html#cx">Cx</a>, root: <a href="sigil-kernel.html#nodeid">NodeId</a>) -&gt; Result&lt;(), <a href="#layouterror">LayoutError</a>&gt;</code> |  |
| <code>const take_republished(&amp;mut self) -&gt; bool</code> |  |
| <code>with_text_measure(measurer: Box&lt;dyn <a href="#textmeasure">TextMeasure</a>&gt;) -&gt; Self</code> |  |


### `LayoutResult`

**Methods**

| Method | Description |
| --- | --- |
| <code>const clips(&amp;self) -&gt; bool</code> |  |
| <code>const content_size(&amp;self) -&gt; <a href="sigil-geometry.html#point">Point</a></code> |  |
| <code>const rect(&amp;self) -&gt; <a href="sigil-geometry.html#rect">Rect</a></code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `LayoutStyle`

**Methods**

| Method | Description |
| --- | --- |
| <code>const align_content(self, align_content: <a href="#aligncontent">AlignContent</a>) -&gt; Self</code> |  |
| <code>const align_items(self, align_items: <a href="#alignitems">AlignItems</a>) -&gt; Self</code> |  |
| <code>const align_self(self, align_self: <a href="#alignself">AlignSelf</a>) -&gt; Self</code> |  |
| <code>block() -&gt; Self</code> |  |
| <code>const box_sizing(self, box_sizing: <a href="#boxsizing">BoxSizing</a>) -&gt; Self</code> |  |
| <code>const data(&amp;self) -&gt; &amp;<a href="#styledata">StyleData</a></code> |  |
| <code>const display(self, display: <a href="#display">Display</a>) -&gt; Self</code> |  |
| <code>flex_column() -&gt; Self</code> |  |
| <code>const flex_direction(self, flex_direction: <a href="#flexdirection">FlexDirection</a>) -&gt; Self</code> |  |
| <code>flex_row() -&gt; Self</code> |  |
| <code>const flex_wrap(self, flex_wrap: <a href="#flexwrap">FlexWrap</a>) -&gt; Self</code> |  |
| <code>grid(columns: Vec&lt;<a href="#tracksize">TrackSize</a>&gt;) -&gt; Self</code> |  |
| <code>const grid_area(self, col: u16, row: u16) -&gt; Self</code> |  |
| <code>const grid_column_span(self, span: u16) -&gt; Self</code> |  |
| <code>grid_columns(self, columns: Vec&lt;<a href="#tracksize">TrackSize</a>&gt;) -&gt; Self</code> |  |
| <code>const grid_dense(self) -&gt; Self</code> |  |
| <code>const grid_row_span(self, span: u16) -&gt; Self</code> |  |
| <code>grid_rows(self, rows: Vec&lt;<a href="#tracksize">TrackSize</a>&gt;) -&gt; Self</code> |  |
| <code>const justify_content(self, justify_content: <a href="#justifycontent">JustifyContent</a>) -&gt; Self</code> |  |
| <code>const position(self, position: <a href="#position">Position</a>) -&gt; Self</code> |  |
| <code>with_layout(self, mutate: impl FnOnce(&amp;mut <a href="#styledata">StyleData</a>)) -&gt; Self</code> |  |
| <code>const fill(self) -&gt; Self</code> |  |
| <code>const fill_height(self) -&gt; Self</code> |  |
| <code>const fill_width(self) -&gt; Self</code> |  |
| <code>const fixed_size(self, width: f32, height: f32) -&gt; Self</code> |  |
| <code>flex_basis(self, basis: impl Into&lt;<a href="#dimension">Dimension</a>&gt;) -&gt; Self</code> |  |
| <code>const grow(self, grow: f32) -&gt; Self</code> |  |
| <code>height(self, height: impl Into&lt;<a href="#dimension">Dimension</a>&gt;) -&gt; Self</code> |  |
| <code>max_height(self, value: impl Into&lt;<a href="#dimension">Dimension</a>&gt;) -&gt; Self</code> |  |
| <code>max_size(self, width: impl Into&lt;<a href="#dimension">Dimension</a>&gt;, height: impl Into&lt;<a href="#dimension">Dimension</a>&gt;) -&gt; Self</code> |  |
| <code>max_width(self, value: impl Into&lt;<a href="#dimension">Dimension</a>&gt;) -&gt; Self</code> |  |
| <code>min_height(self, value: impl Into&lt;<a href="#dimension">Dimension</a>&gt;) -&gt; Self</code> |  |
| <code>min_size(self, width: impl Into&lt;<a href="#dimension">Dimension</a>&gt;, height: impl Into&lt;<a href="#dimension">Dimension</a>&gt;) -&gt; Self</code> |  |
| <code>min_width(self, value: impl Into&lt;<a href="#dimension">Dimension</a>&gt;) -&gt; Self</code> |  |
| <code>pane(width: f32, height: f32) -&gt; Self</code> |  |
| <code>const shrink(self, shrink: f32) -&gt; Self</code> |  |
| <code>width(self, width: impl Into&lt;<a href="#dimension">Dimension</a>&gt;) -&gt; Self</code> |  |
| <code>gap(self, gap: impl Into&lt;<a href="#lengthpercentage">LengthPercentage</a>&gt;) -&gt; Self</code> |  |
| <code>gap_x(self, gap: impl Into&lt;<a href="#lengthpercentage">LengthPercentage</a>&gt;) -&gt; Self</code> |  |
| <code>gap_y(self, gap: impl Into&lt;<a href="#lengthpercentage">LengthPercentage</a>&gt;) -&gt; Self</code> |  |
| <code>inset(self, inset: impl Into&lt;<a href="#lengthpercentageauto">LengthPercentageAuto</a>&gt;) -&gt; Self</code> |  |
| <code>inset_bottom(self, bottom: impl Into&lt;<a href="#lengthpercentageauto">LengthPercentageAuto</a>&gt;) -&gt; Self</code> |  |
| <code>inset_left(self, left: impl Into&lt;<a href="#lengthpercentageauto">LengthPercentageAuto</a>&gt;) -&gt; Self</code> |  |
| <code>inset_right(self, right: impl Into&lt;<a href="#lengthpercentageauto">LengthPercentageAuto</a>&gt;) -&gt; Self</code> |  |
| <code>inset_top(self, top: impl Into&lt;<a href="#lengthpercentageauto">LengthPercentageAuto</a>&gt;) -&gt; Self</code> |  |
| <code>margin(self, margin: impl Into&lt;<a href="#lengthpercentageauto">LengthPercentageAuto</a>&gt;) -&gt; Self</code> |  |
| <code>margin_bottom(self, margin: impl Into&lt;<a href="#lengthpercentageauto">LengthPercentageAuto</a>&gt;) -&gt; Self</code> |  |
| <code>margin_left(self, margin: impl Into&lt;<a href="#lengthpercentageauto">LengthPercentageAuto</a>&gt;) -&gt; Self</code> |  |
| <code>margin_right(self, margin: impl Into&lt;<a href="#lengthpercentageauto">LengthPercentageAuto</a>&gt;) -&gt; Self</code> |  |
| <code>margin_top(self, margin: impl Into&lt;<a href="#lengthpercentageauto">LengthPercentageAuto</a>&gt;) -&gt; Self</code> |  |
| <code>margin_x(self, margin: impl Into&lt;<a href="#lengthpercentageauto">LengthPercentageAuto</a>&gt;) -&gt; Self</code> |  |
| <code>margin_y(self, margin: impl Into&lt;<a href="#lengthpercentageauto">LengthPercentageAuto</a>&gt;) -&gt; Self</code> |  |
| <code>const overflow(self, overflow: <a href="#overflow">Overflow</a>) -&gt; Self</code> |  |
| <code>const overflow_x(self, overflow: <a href="#overflow">Overflow</a>) -&gt; Self</code> |  |
| <code>const overflow_y(self, overflow: <a href="#overflow">Overflow</a>) -&gt; Self</code> |  |
| <code>padding(self, padding: impl Into&lt;<a href="#lengthpercentage">LengthPercentage</a>&gt;) -&gt; Self</code> |  |
| <code>padding_bottom(self, padding: impl Into&lt;<a href="#lengthpercentage">LengthPercentage</a>&gt;) -&gt; Self</code> |  |
| <code>padding_left(self, padding: impl Into&lt;<a href="#lengthpercentage">LengthPercentage</a>&gt;) -&gt; Self</code> |  |
| <code>padding_right(self, padding: impl Into&lt;<a href="#lengthpercentage">LengthPercentage</a>&gt;) -&gt; Self</code> |  |
| <code>padding_top(self, padding: impl Into&lt;<a href="#lengthpercentage">LengthPercentage</a>&gt;) -&gt; Self</code> |  |
| <code>padding_x(self, padding: impl Into&lt;<a href="#lengthpercentage">LengthPercentage</a>&gt;) -&gt; Self</code> |  |
| <code>padding_y(self, padding: impl Into&lt;<a href="#lengthpercentage">LengthPercentage</a>&gt;) -&gt; Self</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>


### `ReactivePosition`

**Methods**

| Method | Description |
| --- | --- |
| <code>get(&amp;self) -&gt; (f32, f32)</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Copy</code>


### `Rect`

**Methods**

| Method | Description |
| --- | --- |
| <code>pub const fn <a href="#rect">Rect</a>&lt;T&gt;::splat(value: T) -&gt; Self</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>bottom: T</code> |  |
| <code>left: T</code> |  |
| <code>right: T</code> |  |
| <code>top: T</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>StructuralPartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>


### `Size`

**Methods**

| Method | Description |
| --- | --- |
| <code>pub const fn <a href="#size">Size</a>&lt;T&gt;::new(width: T, height: T) -&gt; Self</code> |  |
| <code>pub const fn <a href="#size">Size</a>&lt;T&gt;::splat(value: T) -&gt; Self</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>height: T</code> |  |
| <code>width: T</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>StructuralPartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>


### `StyleData`

**Fields**

| Field | Description |
| --- | --- |
| <code>align_content: Option&lt;<a href="#aligncontent">AlignContent</a>&gt;</code> |  |
| <code>align_items: Option&lt;<a href="#alignitems">AlignItems</a>&gt;</code> |  |
| <code>align_self: Option&lt;<a href="#alignself">AlignSelf</a>&gt;</code> |  |
| <code>border: <a href="#rect">Rect</a>&lt;<a href="#lengthpercentage">LengthPercentage</a>&gt;</code> |  |
| <code>box_sizing: <a href="#boxsizing">BoxSizing</a></code> |  |
| <code>direction: <a href="sigil-style.html#direction">Direction</a></code> |  |
| <code>display: <a href="#display">Display</a></code> |  |
| <code>flex_basis: <a href="#dimension">Dimension</a></code> |  |
| <code>flex_direction: <a href="#flexdirection">FlexDirection</a></code> |  |
| <code>flex_grow: f32</code> |  |
| <code>flex_shrink: f32</code> |  |
| <code>flex_wrap: <a href="#flexwrap">FlexWrap</a></code> |  |
| <code>gap: <a href="#size">Size</a>&lt;<a href="#lengthpercentage">LengthPercentage</a>&gt;</code> |  |
| <code>grid_auto_flow_dense: bool</code> |  |
| <code>grid_column_span: u16</code> |  |
| <code>grid_column_start: u16</code> |  |
| <code>grid_row_span: u16</code> |  |
| <code>grid_row_start: u16</code> |  |
| <code>grid_template_columns: Vec&lt;<a href="#tracksize">TrackSize</a>&gt;</code> |  |
| <code>grid_template_rows: Vec&lt;<a href="#tracksize">TrackSize</a>&gt;</code> |  |
| <code>inset: <a href="#rect">Rect</a>&lt;<a href="#lengthpercentageauto">LengthPercentageAuto</a>&gt;</code> |  |
| <code>justify_content: Option&lt;<a href="#justifycontent">JustifyContent</a>&gt;</code> |  |
| <code>margin: <a href="#rect">Rect</a>&lt;<a href="#lengthpercentageauto">LengthPercentageAuto</a>&gt;</code> |  |
| <code>max_size: <a href="#size">Size</a>&lt;<a href="#dimension">Dimension</a>&gt;</code> |  |
| <code>min_size: <a href="#size">Size</a>&lt;<a href="#dimension">Dimension</a>&gt;</code> |  |
| <code>order: i32</code> |  |
| <code>overflow: <a href="#size">Size</a>&lt;<a href="#overflow">Overflow</a>&gt;</code> |  |
| <code>padding: <a href="#rect">Rect</a>&lt;<a href="#lengthpercentage">LengthPercentage</a>&gt;</code> |  |
| <code>position: <a href="#position">Position</a></code> |  |
| <code>size: <a href="#size">Size</a>&lt;<a href="#dimension">Dimension</a>&gt;</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


## Enums

### `AlignContent`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Center</code> |  |
| <code>End</code> |  |
| <code>SpaceAround</code> |  |
| <code>SpaceBetween</code> |  |
| <code>SpaceEvenly</code> |  |
| <code>Start</code> |  |
| <code>Stretch</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `AlignItems`

**Methods**

| Method | Description |
| --- | --- |
| <code>const FlexEnd: Self</code> |  |
| <code>const FlexStart: Self</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>Baseline</code> |  |
| <code>Center</code> |  |
| <code>End</code> |  |
| <code>Start</code> |  |
| <code>Stretch</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `BoxSizing`

**Variants**

| Variant | Description |
| --- | --- |
| <code>BorderBox</code> |  |
| <code>ContentBox</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Dimension`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Auto</code> |  |
| <code>Fill</code> |  |
| <code>Percent(f32)</code> |  |
| <code>Px(f32)</code> |  |
| <code>Text(<a href="#textlength">TextLength</a>)</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>From&lt;f32&gt;</code>, <code>From&lt;<a href="#textlength">TextLength</a>&gt;</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Display`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Block</code> |  |
| <code>Flex</code> |  |
| <code>Grid</code> |  |
| <code>None</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `FlexDirection`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Column</code> |  |
| <code>ColumnReverse</code> |  |
| <code>Row</code> |  |
| <code>RowReverse</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `FlexWrap`

**Variants**

| Variant | Description |
| --- | --- |
| <code>NoWrap</code> |  |
| <code>Wrap</code> |  |
| <code>WrapReverse</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `JustifyContent`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Center</code> |  |
| <code>End</code> |  |
| <code>SpaceAround</code> |  |
| <code>SpaceBetween</code> |  |
| <code>SpaceEvenly</code> |  |
| <code>Start</code> |  |
| <code>Stretch</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `LayoutError`

**Variants**

| Variant | Description |
| --- | --- |
| <code>LayoutInvalid(String)</code> |  |
| <code>NodeMissing(<a href="sigil-kernel.html#nodeid">NodeId</a>)</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Error</code>, <code>Debug</code>, <code>Display</code>, <code>StructuralPartialEq</code>


### `LengthPercentage`

**Methods**

| Method | Description |
| --- | --- |
| <code>const length(value: f32) -&gt; Self</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>Percent(f32)</code> |  |
| <code>Px(f32)</code> |  |
| <code>Text(<a href="#textlength">TextLength</a>)</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>From&lt;f32&gt;</code>, <code>From&lt;<a href="#textlength">TextLength</a>&gt;</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `LengthPercentageAuto`

**Methods**

| Method | Description |
| --- | --- |
| <code>const length(value: f32) -&gt; Self</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>Auto</code> |  |
| <code>Percent(f32)</code> |  |
| <code>Px(f32)</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>From&lt;f32&gt;</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Overflow`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Hidden</code> |  |
| <code>Scroll</code> |  |
| <code>Visible</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Position`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Absolute</code> |  |
| <code>Relative</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `TextLength`

**Methods**

| Method | Description |
| --- | --- |
| <code>px(self) -&gt; f32</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>Cols(f32)</code> |  |
| <code>Em(f32)</code> |  |
| <code>Rows(f32)</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>, <code><a href="#intopx">IntoPx</a></code>


### `TrackSize`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Auto</code> |  |
| <code>Fr(f32)</code> |  |
| <code>MinMax(f32, f32)</code> |  |
| <code>Px(f32)</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


## Traits

### `IntoPx`

**Methods**

| Method | Description |
| --- | --- |
| <code>into_px(self) -&gt; f32</code> |  |


### `LayoutBuilderExt`

**Methods**

| Method | Description |
| --- | --- |
| <code>asymmetric(self, reason: &amp;'static str) -&gt; Self</code> |  |
| <code>layout(self, style: <a href="#layoutstyle">LayoutStyle</a>) -&gt; Self</code> |  |


### `LayoutReactiveExt`

**Methods**

| Method | Description |
| --- | --- |
| <code>bounds_reactive(self, bounds: <a href="sigil-reactive.html#signal">Signal</a>&lt;(f32, f32, f32, f32)&gt;) -&gt; Self</code> |  |
| <code>position_reactive(self, pos: <a href="sigil-reactive.html#signal">Signal</a>&lt;(f32, f32)&gt;) -&gt; Self</code> |  |
| <code>size_reactive(self, size: <a href="sigil-reactive.html#signal">Signal</a>&lt;(f32, f32)&gt;) -&gt; Self</code> |  |


### `TextGeometry`

**Methods**

| Method | Description |
| --- | --- |
| <code>caret(&amp;self, text: &amp;str, style: &amp;<a href="sigil-style.html#textstyle">TextStyle</a>, width: Option&lt;f32&gt;, position: <a href="sigil-hal.html#hitposition">HitPosition</a>) -&gt; <a href="#caretgeometry">CaretGeometry</a></code> |  |
| <code>hit(&amp;self, text: &amp;str, style: &amp;<a href="sigil-style.html#textstyle">TextStyle</a>, width: Option&lt;f32&gt;, point: <a href="sigil-geometry.html#point">Point</a>) -&gt; <a href="sigil-hal.html#hitposition">HitPosition</a></code> |  |
| <code>selection_rects(&amp;self, text: &amp;str, style: &amp;<a href="sigil-style.html#textstyle">TextStyle</a>, width: Option&lt;f32&gt;, selection: (<a href="sigil-hal.html#hitposition">HitPosition</a>, <a href="sigil-hal.html#hitposition">HitPosition</a>)) -&gt; Vec&lt;<a href="sigil-geometry.html#rect">Rect</a>&gt;</code> |  |


### `TextMeasure`

**Methods**

| Method | Description |
| --- | --- |
| <code>begin_frame(&amp;self)</code> |  |
| <code>font_generation(&amp;self) -&gt; u64</code> |  |
| <code>measure(&amp;self, content: &amp;<a href="sigil-kernel.html#textcontent">TextContent</a>, space: <a href="sigil-geometry.html#size">Size</a>) -&gt; <a href="sigil-geometry.html#logicalsize">LogicalSize</a></code> |  |
| <code>measure_styled(&amp;self, content: &amp;<a href="sigil-kernel.html#textcontent">TextContent</a>, style: &amp;<a href="sigil-style.html#textstyle">TextStyle</a>, space: <a href="sigil-geometry.html#size">Size</a>) -&gt; <a href="sigil-geometry.html#logicalsize">LogicalSize</a></code> |  |


## Functions

### `cols`

<pre>pub const fn <a href="#cols">cols</a>(count: f32) -&gt; <a href="#textlength">TextLength</a></pre>


### `column`

<pre>pub fn <a href="#column">column</a>() -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `em`

<pre>pub const fn <a href="#em">em</a>(count: f32) -&gt; <a href="#textlength">TextLength</a></pre>


### `row`

<pre>pub fn <a href="#row">row</a>() -&gt; <a href="sigil-kernel.html#nodebuilder">NodeBuilder</a></pre>


### `rows`

<pre>pub const fn <a href="#rows">rows</a>(count: f32) -&gt; <a href="#textlength">TextLength</a></pre>


## Type aliases

### `AlignSelf`

<pre>pub type <a href="#alignself">AlignSelf</a> = <a href="#alignitems">AlignItems</a></pre>


---

Generated by truesight from `sigil-ui` 0.1.0. See the [API overview](index.md).
