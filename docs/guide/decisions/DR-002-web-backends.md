---
id: DR-002
status: accepted
covers: Web rendering backends and where web tasks run
relates: [docs/guide/web.md]
---

# DR-002: WebGPU with a CPU floor; tasks stay on the page thread

**Status:** Accepted

## Context

On the web, Sigil renders with WebGPU and falls back to the CPU renderer when the browser has no WebGPU adapter. Two additions were considered: a WebGL2 backend between the two, and running blocking tasks in a Web Worker.

WebGL2 cannot run the current GPU pipelines. The fill-paint and text pipelines bind storage buffers, and WebGL2 allows none (`max_storage_buffers_per_shader_stage` is 0 in wgpu's WebGL2 limits). A WebGL2 backend would need a second set of pipelines that move that data through textures or uniforms, and every new paint feature would need both.

`tasks::blocking` and `spawn` take any closure or future. A closure cannot cross `postMessage` to a worker that does not share the page's wasm memory. Sharing memory needs the atomics target features, rebuilding the standard library, COOP and COEP headers on every page that hosts the app, and a second module load in the worker.

## Decision

1. The web backends are WebGPU and the CPU renderer. Sigil does not carry a WebGL2 path.
2. Tasks on the web run on the page thread. `tasks::blocking` keeps its signature on every target; on the web it runs the work without blocking the frame loop longer than the work itself.

## Consequences

- One set of GPU pipelines. Paint features land once.
- Browsers without WebGPU get correct output from the CPU renderer at lower frame rates on large scenes.
- Long CPU work on the web holds the page thread; apps split it into steps or move it off the page with their own worker.
- Revisit when the CPU renderer is too slow for a target app on a browser without WebGPU, or when shared-memory wasm needs no special headers.
