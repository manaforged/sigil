# Module `sigil::animation_runtime`

| Item | Kind | Description |
| --- | --- | --- |
| [`Animator`](#animator) | struct |  |
| [`AttachedAnimator`](#attachedanimator) | struct |  |
| [`AttachedAnimators`](#attachedanimators) | struct |  |
| [`AttachedColorAnimator`](#attachedcoloranimator) | struct |  |
| [`MotionPrefs`](#motionprefs) | struct |  |
| [`Playback`](#playback) | struct |  |
| [`AnimationConfig`](#animationconfig) | enum |  |
| [`AnimatorStatus`](#animatorstatus) | enum |  |
| [`active_count`](#active_count) | fn |  |
| [`config`](#config) | fn |  |
| [`install_animators`](#install_animators) | fn |  |
| [`install_animators_themed`](#install_animators_themed) | fn |  |
| [`lerp_rgba8`](#lerp_rgba8) | fn |  |
| [`linear_channel_to_srgb`](#linear_channel_to_srgb) | fn |  |
| [`srgb_channel_to_linear`](#srgb_channel_to_linear) | fn |  |
| [`tick`](#tick) | fn |  |

## Structs

### `Animator`

**Methods**

| Method | Description |
| --- | --- |
| <code>cancel(&amp;mut self)</code> |  |
| <code>finish(&amp;mut self)</code> |  |
| <code>new(signal: <a href="sigil-reactive.html#signal">Signal</a>&lt;f32&gt;, config: <a href="#animationconfig">AnimationConfig</a>) -&gt; Self</code> |  |
| <code>pause(&amp;mut self)</code> |  |
| <code>resume(&amp;mut self)</code> |  |
| <code>settled(&amp;self) -&gt; bool</code> |  |
| <code>target(&amp;mut self, value: f32)</code> |  |
| <code>value(&amp;self) -&gt; f32</code> |  |

**Trait implementations:** <code>Drop</code>


### `AttachedAnimator`

**Fields**

| Field | Description |
| --- | --- |
| <code>animator: <a href="#animator">Animator</a></code> |  |
| <code>kind: <a href="sigil-style.html#numericatomkind">NumericAtomKind</a></code> |  |
| <code>signal: <a href="sigil-reactive.html#signal">Signal</a>&lt;f32&gt;</code> |  |


### `AttachedAnimators`

**Methods**

| Method | Description |
| --- | --- |
| <code>empty() -&gt; Self</code> |  |
| <code>const is_empty(&amp;self) -&gt; bool</code> |  |
| <code>iter(&amp;self) -&gt; impl Iterator&lt;Item = &amp;<a href="#attachedanimator">AttachedAnimator</a>&gt;</code> |  |
| <code>const len(&amp;self) -&gt; usize</code> |  |
| <code>retarget_for_state(&amp;mut self, slot: <a href="sigil-style.html#stateslot">StateSlot</a>, base: &amp;[<a href="sigil-style.html#styleatom">StyleAtom</a>])</code> |  |

**Trait implementations:** <code>Default</code>, <code>Debug</code>, <code><a href="sigil-style.html#animatoroverlay">AnimatorOverlay</a></code>


### `AttachedColorAnimator`

**Fields**

| Field | Description |
| --- | --- |
| <code>animators: [<a href="#animator">Animator</a>; 4]</code> |  |
| <code>channels: [<a href="sigil-reactive.html#signal">Signal</a>&lt;f32&gt;; 4]</code> |  |
| <code>kind: <a href="sigil-style.html#coloratomkind">ColorAtomKind</a></code> |  |


### `MotionPrefs`

**Methods**

| Method | Description |
| --- | --- |
| <code>reduced() -&gt; bool</code> |  |
| <code>set(preferred: bool)</code> |  |


### `Playback`

**Methods**

| Method | Description |
| --- | --- |
| <code>cancel(&amp;mut self)</code> |  |
| <code>finish(&amp;mut self)</code> |  |
| <code>finished(&amp;self) -&gt; bool</code> |  |
| <code>new(value: <a href="sigil-reactive.html#signal">Signal</a>&lt;f32&gt;, frames: <a href="sigil-animation.html#keyframes">Keyframes</a>) -&gt; Self</code> |  |
| <code>pause(&amp;mut self)</code> |  |
| <code>restart(&amp;mut self)</code> |  |
| <code>resume(&amp;mut self)</code> |  |
| <code>seek(&amp;mut self, elapsed: Duration)</code> |  |

**Trait implementations:** <code>Drop</code>


## Enums

### `AnimationConfig`

**Fields**

| Field | Description |
| --- | --- |
| <code>Spring::preset: <a href="sigil-animation.html#springpreset">SpringPreset</a></code> |  |
| <code>Tween::easing: <a href="sigil-animation.html#easing">Easing</a></code> |  |
| <code>Tween::ms: u16</code> |  |

**Variants**

| Variant | Description |
| --- | --- |
| <code>Spring</code> |  |
| <code>Tween</code> |  |

**Trait implementations:** <code>Clone</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


### `AnimatorStatus`

**Variants**

| Variant | Description |
| --- | --- |
| <code>Running</code> |  |
| <code>Settled</code> |  |

**Trait implementations:** <code>Clone</code>, <code>Eq</code>, <code>PartialEq</code>, <code>Debug</code>, <code>Copy</code>, <code>StructuralPartialEq</code>


## Functions

### `active_count`

<pre>pub fn <a href="#active_count">active_count</a>() -&gt; usize</pre>


### `config`

<pre>pub const fn <a href="#config">config</a>(id: <a href="sigil-style.html#transitionid">TransitionId</a>) -&gt; Option&lt;<a href="#animationconfig">AnimationConfig</a>&gt;</pre>


### `install_animators`

<pre>pub fn <a href="#install_animators">install_animators</a>(atoms: &amp;[<a href="sigil-style.html#styleatom">StyleAtom</a>], state_style: <a href="sigil-style.html#statestyle">StateStyle</a>, component_id: <a href="sigil-style.html#componentid">ComponentId</a>) -&gt; <a href="#attachedanimators">AttachedAnimators</a></pre>


### `install_animators_themed`

<pre>pub fn <a href="#install_animators_themed">install_animators_themed</a>(atoms: &amp;[<a href="sigil-style.html#styleatom">StyleAtom</a>], state_style: <a href="sigil-style.html#statestyle">StateStyle</a>, component_id: <a href="sigil-style.html#componentid">ComponentId</a>, theme: &amp;<a href="sigil-style.html#theme">Theme</a>) -&gt; <a href="#attachedanimators">AttachedAnimators</a></pre>


### `lerp_rgba8`

<pre>pub fn <a href="#lerp_rgba8">lerp_rgba8</a>(a: <a href="sigil-style.html#rgba8">Rgba8</a>, b: <a href="sigil-style.html#rgba8">Rgba8</a>, t: f32) -&gt; <a href="sigil-style.html#rgba8">Rgba8</a></pre>


### `linear_channel_to_srgb`

<pre>pub fn <a href="#linear_channel_to_srgb">linear_channel_to_srgb</a>(v: f32) -&gt; u8</pre>


### `srgb_channel_to_linear`

<pre>pub fn <a href="#srgb_channel_to_linear">srgb_channel_to_linear</a>(c: u8) -&gt; f32</pre>


### `tick`

<pre>pub fn <a href="#tick">tick</a>(dt: f32)</pre>


---

Generated by truesight from `sigil-ui` 0.1.0. See the [API overview](index.md).
