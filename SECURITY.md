# Security policy

## Reporting a vulnerability

Report suspected vulnerabilities through GitHub's **private vulnerability
reporting** ("Report a vulnerability" on this repository's Security tab).
Do **not** file public issues for security problems — public disclosure before
a fix ships puts every downstream user at risk.

Include: the affected crate(s) and version, a reproduction or the code path in
question, and your assessment of impact.

We aim to acknowledge reports within 72 hours and to agree on a disclosure
timeline with the reporter once a fix is available.

## Scope and posture

- Sigil is pre-1.0 (`0.1.0-alpha.*`). APIs and security-relevant behavior may
  still change between lockstep releases; pin exact versions downstream.
- The workspace denies `unsafe_code` and runs `cargo deny check` in CI
  (licenses, RustSec advisories, banned crates, registry sources) on every
  change that lands.
- Rendering is GPU-backed (wgpu); untrusted input surfaces of note are the
  style-class parser, text shaping of arbitrary strings, and SVG icon baking
  (usvg). If you find a panic, OOB, or unsoundness reachable from those, it is
  in scope.
