# Vega notice

The `vega-lite` markdown fence draws charts with the Vega and Vega-Lite JavaScript libraries, which `sigil-ui` runs in an embedded QuickJS runtime. The two files here are the minified browser builds, unchanged from the npm packages:

| File | Package | Source | SHA-256 |
|------|---------|--------|---------|
| `vega.min.js` | vega 6.4.0, `build/vega.min.js` | <https://registry.npmjs.org/vega/-/vega-6.4.0.tgz> | `8f6a3587cf8d4f42c7e08120e3eb05d067e746d554e39d2dcf52acc0bd5ba28f` |
| `vega-lite.min.js` | vega-lite 6.5.0, `build/vega-lite.min.js` | <https://registry.npmjs.org/vega-lite/-/vega-lite-6.5.0.tgz> | `83e215e84e308ee8688fa6503ba0f1cf8a3c05c0d61df2841a0b92bf86809e00` |

Vega and Vega-Lite are licensed under the BSD 3-Clause License, Copyright (c) 2015-2023 and Copyright (c) 2015, University of Washington Interactive Data Lab. The builds bundle their dependencies, each under its own license:

| Packages | License | Copyright |
|----------|---------|-----------|
| d3-array, d3-color, d3-dispatch, d3-dsv, d3-force, d3-format, d3-hierarchy, d3-interpolate, d3-path, d3-quadtree, d3-scale, d3-shape, d3-time, d3-time-format, d3-timer, internmap | ISC | Mike Bostock |
| d3-geo | ISC; parts from GeographicLib under MIT | Mike Bostock; Charles Karney |
| d3-geo-projection | ISC; parts under MIT | Mike Bostock; Ricky Reusser |
| d3-scale-chromatic | ISC; ColorBrewer schemes under Apache 2.0 | Mike Bostock; Cynthia Brewer, Mark Harrower, and The Pennsylvania State University |
| d3-ease | BSD 3-Clause | Mike Bostock; Robert Penner |
| d3-delaunay | ISC | Observable, Inc.; Mapbox |
| delaunator | ISC | Mapbox |
| topojson-client | ISC | Michael Bostock |
| json-stringify-pretty-compact | MIT | Simon Lydell |
| robust-predicates | Unlicense | Vladimir Agafonkin (public domain dedication) |
| tslib | 0BSD | Microsoft Corporation |

The full license text of each package is in [`LICENSES.txt`](LICENSES.txt), from the versions npm resolves for vega 6.4.0 and vega-lite 6.5.0. Embedding them here does not extend Sigil's own license to them; each remains under its own license.
