# Decision records

Put each architectural decision in a numbered record here: one file per decision, named `DR-<number>-<slug>.md`, committed to the repository. If code or a manifest cites a DR number, the record must exist here. A citation to a document that readers cannot open is a defect.

## Current records

- `DR-001-signals-over-entity.md`: why `Signal` is the author type, not an entity with a render method.
- `DR-002-web-backends.md`: WebGPU with a CPU floor, no WebGL2 path, and tasks on the page thread on the web.
