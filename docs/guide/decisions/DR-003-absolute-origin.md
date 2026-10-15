---
id: DR-003
status: accepted
covers: Where absolutely positioned children start, and what their insets measure against
relates: [docs/guide/quickstart.md]
---

# DR-003: Absolute children keep Sigil's origins; insets on both sides stretch

**Status:** Accepted

## Context

CSS places an absolutely positioned box against its parent's padding box: `left: 0` puts it at the padding edge, and a box with no insets sits where it would have flowed. Sigil starts absolute children at the parent's content origin in block layout and at the border origin in flex layout. Widgets are built on that: the text editor's text layer, the virtual list's scrollbar, toasts, and dropdowns place themselves with insets measured from those origins.

Placing absolute children against the padding edge, as CSS does, moved the text editor's text onto its padding and changed a golden image by 7%. Every widget that positions a layer with insets would need the same review.

## Decision

1. An absolute child in a block parent starts at the parent's content origin; in a flex parent, at the parent's border origin. Insets offset from there.
2. Absolute children are placed after the parent's box is final, so an auto-height parent has a height to measure against.
3. A child with insets on both sides of an axis and no size of its own spans the box its origin belongs to, minus the two insets and its margins: the content box in a block parent, the padding box in a flex parent. A flex child stretched this way gets that size as its own.

## Consequences

- Existing widgets keep their layout.
- A backdrop pinned with `inset(0.0)` covers a block parent's content box exactly, padding or not, and follows the parent as its content grows.
- Code ported from CSS that relies on the padding edge needs an inset that adds the padding.
- Revisit when a widget needs CSS placement; move every widget at once and rebake the goldens.
