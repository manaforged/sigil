# Module `sigil::events`

| Item | Kind | Description |
| --- | --- | --- |
| [`Shortcut`](#shortcut) | struct |  |
| [`ClickConfig`](#clickconfig) | struct |  |
| [`EventHandlers`](#eventhandlers) | struct |  |
| [`EventRouter`](#eventrouter) | struct |  |
| [`FileDropEvent`](#filedropevent) | struct |  |
| [`KeyEvent`](#keyevent) | struct |  |
| [`KeyEventInit`](#keyeventinit) | struct |  |
| [`MouseButtons`](#mousebuttons) | struct |  |
| [`PointerEvent`](#pointerevent) | struct |  |
| [`PointerEventInit`](#pointereventinit) | struct |  |
| [`ScrollEvent`](#scrollevent) | struct |  |
| [`ScrollEventInit`](#scrolleventinit) | struct |  |
| [`SelectableText`](#selectabletext) | struct |  |
| [`FileDropPhase`](#filedropphase) | enum |  |
| [`FocusIntent`](#focusintent) | enum |  |
| [`ImeEvent`](#imeevent) | enum |  |
| [`KeyPhase`](#keyphase) | enum |  |
| [`LogicalKey`](#logicalkey) | enum |  |
| [`MouseButton`](#mousebutton) | enum |  |
| [`PointerKind`](#pointerkind) | enum |  |
| [`PointerPhase`](#pointerphase) | enum |  |
| [`ScrollDelta`](#scrolldelta) | enum |  |
| [`ScrollPhase`](#scrollphase) | enum |  |
| [`ShellEvent`](#shellevent) | enum |  |
| [`WindowEvent`](#windowevent) | enum |  |
| [`NodeEventsExt`](#nodeeventsext) | trait |  |
| [`focus_from_keyboard`](#focus_from_keyboard) | fn |  |
| [`request_focus`](#request_focus) | fn |  |
| [`request_reveal`](#request_reveal) | fn |  |
| [`request_window_command`](#request_window_command) | fn |  |
| [`PRIMARY`](#primary) | const |  |
| [`Key`](#key) | use |  |
| [`ModifiersState`](#modifiersstate) | use |  |
| [`PhysicalKey`](#physicalkey) | use |  |

## Structs

### `Shortcut`

**Methods**

| Method | Description |
| --- | --- |
| <code>ctrl(key: impl Into&lt;<a href="#logicalkey">LogicalKey</a>&gt;) -&gt; Self</code> |  |
| <code>matches(&amp;self, event: &amp;<a href="#keyevent">KeyEvent</a>) -&gt; bool</code> |  |
| <code>new(key: impl Into&lt;<a href="#logicalkey">LogicalKey</a>&gt;, modifiers: Modifiers) -&gt; Self</code> |  |
| <code>plain(key: impl Into&lt;<a href="#logicalkey">LogicalKey</a>&gt;) -&gt; Self</code> |  |
| <code>primary(key: impl Into&lt;<a href="#logicalkey">LogicalKey</a>&gt;) -&gt; Self</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>key: <a href="#logicalkey">LogicalKey</a></code> |  |
| <code>modifiers: Modifiers</code> |  |

**Trait implementations:** <code>From&lt;<a href="#logicalkey">LogicalKey</a>&gt;</code>, <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>From&lt;NamedKey&gt;</code>, <code>Debug</code>, <code>Hash</code>, <code>StructuralPartialEq</code>


### `ClickConfig`

**Methods**

| Method | Description |
| --- | --- |
| <code>current() -&gt; Self</code> |  |
| <code>is_repeat(&amp;self, elapsed: Duration, from: <a href="sigil-geometry.html#point">Point</a>, to: <a href="sigil-geometry.html#point">Point</a>) -&gt; bool</code> |  |
| <code>set(config: Self)</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>distance: f32</code> |  |
| <code>interval: Duration</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `EventHandlers`

**Trait implementations:** <code>Default</code>, <code>Debug</code>


### `EventRouter`

**Methods**

| Method | Description |
| --- | --- |
| <code>a11y_focus_update(&amp;self) -&gt; TreeUpdate</code> |  |
| <code>flush_focus_requests(&amp;mut self, cx: &amp;mut <a href="sigil-kernel.html#cx">Cx</a>, layout: &amp;<a href="sigil-layout.html#layoutengine">LayoutEngine</a>)</code> |  |
| <code>const focus(&amp;self) -&gt; Option&lt;<a href="sigil-kernel.html#nodeid">NodeId</a>&gt;</code> |  |
| <code>focus_if_needed(&amp;mut self, cx: &amp;mut <a href="sigil-kernel.html#cx">Cx</a>, layout: &amp;<a href="sigil-layout.html#layoutengine">LayoutEngine</a>)</code> |  |
| <code>focus_intent(&amp;mut self, cx: &amp;mut <a href="sigil-kernel.html#cx">Cx</a>, layout: &amp;<a href="sigil-layout.html#layoutengine">LayoutEngine</a>, intent: <a href="#focusintent">FocusIntent</a>)</code> |  |
| <code>focus_next(&amp;mut self, cx: &amp;mut <a href="sigil-kernel.html#cx">Cx</a>)</code> |  |
| <code>focus_prev(&amp;mut self, cx: &amp;mut <a href="sigil-kernel.html#cx">Cx</a>)</code> |  |
| <code>is_click_target(&amp;self, cx: &amp;<a href="sigil-kernel.html#cx">Cx</a>, layout: &amp;<a href="sigil-layout.html#layoutengine">LayoutEngine</a>, target: <a href="sigil-kernel.html#nodeid">NodeId</a>) -&gt; bool</code> |  |
| <code>is_focus_target(&amp;mut self, cx: &amp;<a href="sigil-kernel.html#cx">Cx</a>, layout: &amp;<a href="sigil-layout.html#layoutengine">LayoutEngine</a>, target: <a href="sigil-kernel.html#nodeid">NodeId</a>) -&gt; bool</code> |  |
| <code>new() -&gt; Self</code> |  |
| <code>reconcile_hover(&amp;mut self, cx: &amp;<a href="sigil-kernel.html#cx">Cx</a>, layout: &amp;<a href="sigil-layout.html#layoutengine">LayoutEngine</a>)</code> |  |
| <code>route(&amp;mut self, cx: &amp;mut <a href="sigil-kernel.html#cx">Cx</a>, layout: &amp;<a href="sigil-layout.html#layoutengine">LayoutEngine</a>, event: <a href="#shellevent">ShellEvent</a>)</code> |  |
| <code>set_focus(&amp;mut self, cx: &amp;mut <a href="sigil-kernel.html#cx">Cx</a>, target: Option&lt;<a href="sigil-kernel.html#nodeid">NodeId</a>&gt;)</code> |  |
| <code>focus_wants_ime(&amp;self, cx: &amp;<a href="sigil-kernel.html#cx">Cx</a>) -&gt; bool</code> |  |
| <code>ime_caret_rect(&amp;self, cx: &amp;<a href="sigil-kernel.html#cx">Cx</a>, layout: &amp;<a href="sigil-layout.html#layoutengine">LayoutEngine</a>) -&gt; Option&lt;<a href="sigil-geometry.html#rect">Rect</a>&gt;</code> |  |

**Trait implementations:** <code>Default</code>, <code>Debug</code>


### `FileDropEvent`

**Fields**

| Field | Description |
| --- | --- |
| <code>path: Option&lt;String&gt;</code> |  |
| <code>phase: <a href="#filedropphase">FileDropPhase</a></code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


### `KeyEvent`

**Methods**

| Method | Description |
| --- | --- |
| <code>new(init: <a href="#keyeventinit">KeyEventInit</a>) -&gt; Self</code> |  |
| <code>const normalized_modifiers(&amp;self) -&gt; Modifiers</code> |  |
| <code>const propagation_stopped(&amp;self) -&gt; bool</code> |  |
| <code>const stop_propagation(&amp;mut self)</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>key: <a href="#logicalkey">LogicalKey</a></code> |  |
| <code>modifiers: Modifiers</code> |  |
| <code>phase: <a href="#keyphase">KeyPhase</a></code> |  |
| <code>physical: Code</code> |  |
| <code>repeat: bool</code> |  |
| <code>text: Option&lt;SmolStr&gt;</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


### `KeyEventInit`

**Fields**

| Field | Description |
| --- | --- |
| <code>key: <a href="#logicalkey">LogicalKey</a></code> |  |
| <code>modifiers: Modifiers</code> |  |
| <code>phase: <a href="#keyphase">KeyPhase</a></code> |  |
| <code>physical: Code</code> |  |
| <code>repeat: bool</code> |  |
| <code>text: Option&lt;SmolStr&gt;</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


### `MouseButtons`

**Methods**

| Method | Description |
| --- | --- |
| <code>const BACK: Self</code> |  |
| <code>const EMPTY: Self</code> |  |
| <code>const FORWARD: Self</code> |  |
| <code>const LEFT: Self</code> |  |
| <code>const MIDDLE: Self</code> |  |
| <code>const RIGHT: Self</code> |  |
| <code>const contains(self, button: Self) -&gt; bool</code> |  |
| <code>const empty() -&gt; Self</code> |  |
| <code>const insert(&amp;mut self, button: Self)</code> |  |
| <code>const remove(&amp;mut self, button: Self)</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `PointerEvent`

**Methods**

| Method | Description |
| --- | --- |
| <code>const capture_pointer(&amp;mut self)</code> |  |
| <code>const capture_requested(&amp;self) -&gt; bool</code> |  |
| <code>const is_captured(&amp;self) -&gt; bool</code> |  |
| <code>const new(init: <a href="#pointereventinit">PointerEventInit</a>) -&gt; Self</code> |  |
| <code>const propagation_stopped(&amp;self) -&gt; bool</code> |  |
| <code>const set_captured(&amp;mut self, captured: bool)</code> |  |
| <code>const stop_propagation(&amp;mut self)</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>button: Option&lt;<a href="#mousebutton">MouseButton</a>&gt;</code> |  |
| <code>buttons: <a href="#mousebuttons">MouseButtons</a></code> |  |
| <code>delta: <a href="sigil-geometry.html#point">Point</a></code> |  |
| <code>kind: <a href="#pointerkind">PointerKind</a></code> |  |
| <code>modifiers: Modifiers</code> |  |
| <code>phase: <a href="#pointerphase">PointerPhase</a></code> |  |
| <code>position: <a href="sigil-geometry.html#point">Point</a></code> |  |
| <code>target_size: <a href="sigil-geometry.html#logicalsize">LogicalSize</a></code> |  |
| <code>window_position: <a href="sigil-geometry.html#point">Point</a></code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


### `PointerEventInit`

**Fields**

| Field | Description |
| --- | --- |
| <code>button: Option&lt;<a href="#mousebutton">MouseButton</a>&gt;</code> |  |
| <code>buttons: <a href="#mousebuttons">MouseButtons</a></code> |  |
| <code>delta: <a href="sigil-geometry.html#point">Point</a></code> |  |
| <code>kind: <a href="#pointerkind">PointerKind</a></code> |  |
| <code>modifiers: Modifiers</code> |  |
| <code>phase: <a href="#pointerphase">PointerPhase</a></code> |  |
| <code>position: <a href="sigil-geometry.html#point">Point</a></code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `ScrollEvent`

**Methods**

| Method | Description |
| --- | --- |
| <code>const new(init: <a href="#scrolleventinit">ScrollEventInit</a>) -&gt; Self</code> |  |
| <code>const propagation_stopped(&amp;self) -&gt; bool</code> |  |
| <code>const stop_propagation(&amp;mut self)</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>delta: <a href="#scrolldelta">ScrollDelta</a></code> |  |
| <code>modifiers: Modifiers</code> |  |
| <code>phase: <a href="#scrollphase">ScrollPhase</a></code> |  |
| <code>position: <a href="sigil-geometry.html#point">Point</a></code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


### `ScrollEventInit`

**Fields**

| Field | Description |
| --- | --- |
| <code>delta: <a href="#scrolldelta">ScrollDelta</a></code> |  |
| <code>modifiers: Modifiers</code> |  |
| <code>phase: <a href="#scrollphase">ScrollPhase</a></code> |  |
| <code>position: <a href="sigil-geometry.html#point">Point</a></code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `SelectableText`


## Enums

### `FileDropPhase`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Cancelled</code> |  |
| <code>Dropped</code> |  |
| <code>Hovered</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `FocusIntent`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Down</code> |  |
| <code>Left</code> |  |
| <code>Right</code> |  |
| <code>Up</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `ImeEvent`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Commit(SmolStr)</code> |  |
| <code>Disabled</code> |  |
| <code>Enabled</code> |  |
| <code>Preedit(SmolStr, Option&lt;(usize, usize)&gt;)</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


### `KeyPhase`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Down</code> |  |
| <code>Up</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `LogicalKey`

**Methods**

| Method | Description |
| --- | --- |
| <code>character(value: impl Into&lt;SmolStr&gt;) -&gt; Self</code> |  |
| <code>const named(key: NamedKey) -&gt; Self</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>Character(SmolStr)</code> |  |
| <code>Named(NamedKey)</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>From&lt;NamedKey&gt;</code>, <code>From&lt;SmolStr&gt;</code>, <code>Debug</code>, <code>Hash</code>, <code>StructuralPartialEq</code>


### `MouseButton`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Back</code> |  |
| <code>Forward</code> |  |
| <code>Left</code> |  |
| <code>Middle</code> |  |
| <code>Other(u16)</code> |  |
| <code>Right</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `PointerKind`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Mouse</code> |  |
| <code>Pen</code> |  |
| <code>Touch</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `PointerPhase`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Cancel</code> |  |
| <code>Down</code> |  |
| <code>Enter</code> |  |
| <code>Leave</code> |  |
| <code>Move</code> |  |
| <code>Up</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `ScrollDelta`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Lines(<a href="sigil-geometry.html#point">Point</a>)</code> |  |
| <code>Pixels(<a href="sigil-geometry.html#point">Point</a>)</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `ScrollPhase`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Cancelled</code> |  |
| <code>Ended</code> |  |
| <code>Moved</code> |  |
| <code>Started</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `ShellEvent`

**Variants**

| Variant | Description |
| --- | --- |
| <code>FileDrop(<a href="#filedropevent">FileDropEvent</a>)</code> |  |
| <code>Ime(<a href="#imeevent">ImeEvent</a>)</code> |  |
| <code>Key(<a href="#keyevent">KeyEvent</a>)</code> |  |
| <code>Pointer(<a href="#pointerevent">PointerEvent</a>)</code> |  |
| <code>Scroll(<a href="#scrollevent">ScrollEvent</a>)</code> |  |
| <code>Window(<a href="#windowevent">WindowEvent</a>)</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


### `WindowEvent`

**Variants**

| Variant | Description |
| --- | --- |
| <code>CloseRequested</code> |  |
| <code>Destroyed</code> |  |
| <code>Focused(bool)</code> |  |
| <code>Occluded(bool)</code> |  |
| <code>Resized(<a href="sigil-geometry.html#physicalsize">PhysicalSize</a>)</code> |  |
| <code>ScaleFactorChanged(<a href="sigil-geometry.html#scale">Scale</a>)</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>StructuralPartialEq</code>


## Traits

### `NodeEventsExt`

**Methods**

| Method | Description |
| --- | --- |
| <code>autofocus(self) -&gt; Self</code> |  |
| <code>file_drop&lt;H&gt;(self, h: H) -&gt; Self where H: Fn(&amp;<a href="#filedropevent">FileDropEvent</a>) + 'static</code> |  |
| <code>focus_in&lt;H&gt;(self, h: H) -&gt; Self where H: Fn() + 'static</code> |  |
| <code>focus_out&lt;H&gt;(self, h: H) -&gt; Self where H: Fn() + 'static</code> |  |
| <code>focus_when(self, trigger: <a href="sigil-reactive.html#signal">Signal</a>&lt;u64&gt;) -&gt; Self</code> |  |
| <code>ime_caret(self, anchor: Rc&lt;Cell&lt;Option&lt;<a href="sigil-layout.html#caretgeometry">CaretGeometry</a>&gt;&gt;&gt;) -&gt; Self</code> |  |
| <code>ime_eligible&lt;F&gt;(self, f: F) -&gt; Self where F: Fn() -&gt; bool + 'static</code> |  |
| <code>key&lt;S, H&gt;(self, shortcut: S, h: H) -&gt; Self where S: Into&lt;<a href="#shortcut">Shortcut</a>&gt;, H: Fn(&amp;mut <a href="#keyevent">KeyEvent</a>) + 'static</code> |  |
| <code>key_any&lt;H&gt;(self, h: H) -&gt; Self where H: Fn(&amp;mut <a href="#keyevent">KeyEvent</a>) + 'static</code> |  |
| <code>key_capture&lt;S, H&gt;(self, shortcut: S, h: H) -&gt; Self where S: Into&lt;<a href="#shortcut">Shortcut</a>&gt;, H: Fn(&amp;mut <a href="#keyevent">KeyEvent</a>) + 'static</code> |  |
| <code>key_capture_any&lt;H&gt;(self, h: H) -&gt; Self where H: Fn(&amp;mut <a href="#keyevent">KeyEvent</a>) + 'static</code> |  |
| <code>on_ime&lt;H&gt;(self, h: H) -&gt; Self where H: Fn(&amp;<a href="#imeevent">ImeEvent</a>) + 'static</code> |  |
| <code>on_scroll&lt;H&gt;(self, h: H) -&gt; Self where H: Fn(&amp;mut <a href="#scrollevent">ScrollEvent</a>) + 'static</code> |  |
| <code>pointer_down&lt;H&gt;(self, h: H) -&gt; Self where H: Fn(&amp;mut <a href="#pointerevent">PointerEvent</a>) + 'static</code> |  |
| <code>pointer_down_capture&lt;H&gt;(self, h: H) -&gt; Self where H: Fn(&amp;mut <a href="#pointerevent">PointerEvent</a>) + 'static</code> |  |
| <code>pointer_enter&lt;H&gt;(self, h: H) -&gt; Self where H: Fn(&amp;mut <a href="#pointerevent">PointerEvent</a>) + 'static</code> |  |
| <code>pointer_leave&lt;H&gt;(self, h: H) -&gt; Self where H: Fn(&amp;mut <a href="#pointerevent">PointerEvent</a>) + 'static</code> |  |
| <code>pointer_move&lt;H&gt;(self, h: H) -&gt; Self where H: Fn(&amp;mut <a href="#pointerevent">PointerEvent</a>) + 'static</code> |  |
| <code>pointer_move_capture&lt;H&gt;(self, h: H) -&gt; Self where H: Fn(&amp;mut <a href="#pointerevent">PointerEvent</a>) + 'static</code> |  |
| <code>pointer_up&lt;H&gt;(self, h: H) -&gt; Self where H: Fn(&amp;mut <a href="#pointerevent">PointerEvent</a>) + 'static</code> |  |
| <code>pointer_up_capture&lt;H&gt;(self, h: H) -&gt; Self where H: Fn(&amp;mut <a href="#pointerevent">PointerEvent</a>) + 'static</code> |  |
| <code>selectable_text(self, yes: bool) -&gt; Self</code> |  |
| <code>shortcut&lt;S, H&gt;(self, shortcut: S, h: H) -&gt; Self where S: Into&lt;<a href="#shortcut">Shortcut</a>&gt;, H: Fn(&amp;mut <a href="#keyevent">KeyEvent</a>) + 'static</code> |  |


## Functions

### `focus_from_keyboard`

<pre>pub fn <a href="#focus_from_keyboard">focus_from_keyboard</a>() -&gt; bool</pre>


### `request_focus`

<pre>pub fn <a href="#request_focus">request_focus</a>(id: <a href="sigil-kernel.html#nodeid">NodeId</a>)</pre>


### `request_reveal`

<pre>pub fn <a href="#request_reveal">request_reveal</a>(id: <a href="sigil-kernel.html#nodeid">NodeId</a>)</pre>


### `request_window_command`

<pre>pub fn <a href="#request_window_command">request_window_command</a>(command: <a href="sigil-app.html#windowcommand">WindowCommand</a>)</pre>


## Constants

### `PRIMARY`

<pre>pub const <a href="#primary">PRIMARY</a>: Modifiers</pre>


## Re-exports

### `Key`


### `ModifiersState`


### `PhysicalKey`


---

Generated by truesight from `sigil-ui` 0.1.0. See the [API overview](index.md).
