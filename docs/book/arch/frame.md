# Frame loop

`sigil::launch` drives one phase sequence per frame:

```text
Idle → Effect → Layout → Paint → Idle
```

Writes that change the tree are legal in Effect. Paint is
read-mostly. `Signal::set` during Paint panics in debug.

The type is `DrawPhase` on `sigil_reactive::Runtime`.

## One frame (`frame_once`)

Order in `sigil-app` (names are the production functions):

1. Drain a watched theme file, if any.
2. Pump `spawn` completions onto the reactive completion queue.
3. Install a pending native menu or tray (first frame only).
4. **`Runtime::begin_effect`**
   - Tick intervals.
   - Ensure the wgpu surface exists.
   - Pump menu / tray events onto `EventChannel`.
   - Drain OS events through `EventRouter`. Handlers `signal.set`.
   - Tick animations while writes are still legal.
5. **`Runtime::begin_layout`**
   - If the frame is dirty, `LayoutEngine::compute`.
   - Drain AccessKit actions against this frame's boxes.
   - Sync IME allowed/denied from focus.
6. **`Runtime::begin_paint`**
   - If dirty (or the shell forced a paint), emit and submit the
     `CommandScene`.
   - Project the kernel tree to AccessKit.
7. **`Runtime::end_frame`** — replay any deferred paint-phase writes,
   then Idle.

Hit-testing uses **last frame's** boxes. That is the layout the user
saw. This frame's layout reflects the writes from this frame's
handlers, and is what the next hit-test will see.

## Dirty

A signal write with live subscribers sets `Runtime::frame_dirty`.
Layout and paint skip when the flag is clear and the shell did not
force a paint (expose / resize). Animation can also request a
successor redraw without a signal write.

Idle windows do not spin the frame loop. The waker from `spawn`,
menu, tray, or input starts the next frame.

## Why phases exist

Without a paint fence, a `derived` that writes during emit can
invalidate the scene it is painting. The fence is the same idea as
GPUI's "don't mutate during paint," implemented as a phase on the
signal runtime rather than as a method on an entity context.

Convention: do not write during Layout either. The runtime does not
panic there today. Effect is the write phase.
