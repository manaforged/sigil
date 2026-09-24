# Platform

`sigil::shell` owns the winit window and the event loop. `run`
drives the phases and presents the window. `Host` embeds the same tree
in a loop you own.

## Input

OS events arrive from the shell. `EventRouter` hit-tests the boxes from
last frame, because that is the layout the user saw. A handler writes a
signal, and the next frame reflects the change. The node that calls
`capture()` owns the pointer, not the deepest child below the cursor.
Focus, IME enablement, and pointer capture attach to the same tree.

## Accessibility

Every kernel node carries `A11yProps`: role, name, state signals, and a
live region. The `a11y` module builds an `accesskit::TreeUpdate` from
those nodes; widgets do not keep a second tree in sync. The bridge
remaps on paint and peeks live state at that moment. AT actions, such
as focus and click, drain in the Layout phase against this frame's
boxes, so their writes are visible in the same paint.

Headless tests call the `a11y` tree function with the `Cx`, the focused
node, and a bounds lookup, then assert roles and names.

## Profiling

Set `SIGIL_PROFILE` to collect a trace. `SIGIL_PROFILE=1` writes
`sigil-profile.json`; any other value is a path. The trace is Chrome
trace JSON. The phase table reports per-phase time. Counters include
`scene.cmds` and `scene.glyphs`.

## Terminal host

The module `sigil::terminal` (feature `terminal`) is the terminal host.
`sigil::run` with `.terminal()` opens the terminal, rasterizes the same
`Host` tree onto cells, and handles terminal key and mouse events. The
feature does not depend on wgpu or winit.
