# Module `sigil::animation`

| Item | Kind | Description |
| --- | --- | --- |
| [`CubicBezier`](#cubicbezier) | struct |  |
| [`Keyframe`](#keyframe) | struct |  |
| [`Keyframes`](#keyframes) | struct |  |
| [`Spring`](#spring) | struct |  |
| [`Tween`](#tween) | struct |  |
| [`AnimationDirection`](#animationdirection) | enum |  |
| [`AnimationFill`](#animationfill) | enum |  |
| [`Easing`](#easing) | enum |  |
| [`SpringPreset`](#springpreset) | enum |  |

## Structs

### `CubicBezier`

**Methods**

| Method | Description |
| --- | --- |
| <code>new(x1: f32, y1: f32, x2: f32, y2: f32) -&gt; Self</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Keyframe`

**Methods**

| Method | Description |
| --- | --- |
| <code>const easing(self, easing: <a href="#easing">Easing</a>) -&gt; Self</code> |  |
| <code>new(offset: f32, value: f32) -&gt; Self</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Keyframes`

**Methods**

| Method | Description |
| --- | --- |
| <code>const delay(self, delay: Duration) -&gt; Self</code> |  |
| <code>const direction(self, direction: <a href="#animationdirection">AnimationDirection</a>) -&gt; Self</code> |  |
| <code>const fill(self, fill: <a href="#animationfill">AnimationFill</a>) -&gt; Self</code> |  |
| <code>new(frames: impl IntoIterator&lt;Item = <a href="#keyframe">Keyframe</a>&gt;, duration: Duration) -&gt; Self</code> |  |
| <code>const repeat(self, iterations: NonZeroU32) -&gt; Self</code> |  |
| <code>const repeat_forever(self) -&gt; Self</code> |  |
| <code>sample(&amp;self, progress: f32) -&gt; f32</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Debug</code>


### `Spring`

**Methods**

| Method | Description |
| --- | --- |
| <code>const DEFAULT_VALUE_EPSILON: f32</code> |  |
| <code>const DEFAULT_VELOCITY_EPSILON: f32</code> |  |
| <code>from_preset(initial: f32, preset: <a href="#springpreset">SpringPreset</a>) -&gt; Self</code> |  |
| <code>is_settled(&amp;self) -&gt; bool</code> |  |
| <code>new(initial: f32, stiffness: f32, damping: f32, mass: f32) -&gt; Self</code> |  |
| <code>const reset(&amp;mut self, value: f32)</code> |  |
| <code>const set_target(&amp;mut self, target: f32)</code> |  |
| <code>step(&amp;mut self, dt: f32)</code> |  |
| <code>const target(&amp;self) -&gt; f32</code> |  |
| <code>const value(&amp;self) -&gt; f32</code> |  |
| <code>const velocity(&amp;self) -&gt; f32</code> |  |
| <code>with_epsilons(self, value: f32, velocity: f32) -&gt; Self</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Debug</code>, <code>Copy</code>


### `Tween`

**Methods**

| Method | Description |
| --- | --- |
| <code>const duration(&amp;self) -&gt; f32</code> |  |
| <code>const from(&amp;self) -&gt; f32</code> |  |
| <code>is_finished(&amp;self) -&gt; bool</code> |  |
| <code>new(from: f32, to: f32, duration: f32) -&gt; Self</code> |  |
| <code>progress(&amp;self) -&gt; f32</code> |  |
| <code>const restart(&amp;mut self)</code> |  |
| <code>step(&amp;mut self, dt: f32)</code> |  |
| <code>const to(&amp;self) -&gt; f32</code> |  |
| <code>value(&amp;self) -&gt; f32</code> |  |
| <code>const with_easing(self, easing: <a href="#easing">Easing</a>) -&gt; Self</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Debug</code>, <code>Copy</code>


## Enums

### `AnimationDirection`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Alternate</code> |  |
| <code>AlternateReverse</code> |  |
| <code>Normal</code> |  |
| <code>Reverse</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `AnimationFill`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Backwards</code> |  |
| <code>Both</code> |  |
| <code>Forwards</code> |  |
| <code>None</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `Easing`

**Methods**

| Method | Description |
| --- | --- |
| <code>apply(self, t: f32) -&gt; f32</code> |  |
| <code>const steps(count: NonZeroU16, start: bool) -&gt; Self</code> |  |

**Fields**

| Field | Description |
| --- | --- |
| <code>Steps::count: NonZeroU16</code> |  |
| <code>Steps::start: bool</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>BackInOut</code> |  |
| <code>BackOut</code> |  |
| <code>Bezier(<a href="#cubicbezier">CubicBezier</a>)</code> |  |
| <code>BounceOut</code> |  |
| <code>CubicIn</code> |  |
| <code>CubicInOut</code> |  |
| <code>CubicOut</code> |  |
| <code>ExpoIn</code> |  |
| <code>ExpoOut</code> |  |
| <code>Linear</code> |  |
| <code>QuadIn</code> |  |
| <code>QuadInOut</code> |  |
| <code>QuadOut</code> |  |
| <code>SineIn</code> |  |
| <code>SineInOut</code> |  |
| <code>SineOut</code> |  |
| <code>Steps</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Default</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `SpringPreset`

**Methods**

| Method | Description |
| --- | --- |
| <code>const parameters(self) -&gt; (f32, f32, f32)</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>Bouncy</code> |  |
| <code>Gentle</code> |  |
| <code>Smooth</code> |  |
| <code>Snappy</code> |  |
| <code>Stiff</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Hash</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


---

Generated by truesight from `sigil-ui` 0.1.0. See the [API overview](index.md).
