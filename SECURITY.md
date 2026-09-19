# Security

Report vulnerabilities privately to [thomas@manaforge.tech](mailto:thomas@manaforge.tech).
When GitHub private reporting is available, you can also use the
repository's Security tab and select "Report a vulnerability".
Do not open a public issue.

## Supported versions

The latest `0.x` release gets security fixes. Older `0.x` releases do
not.

The untrusted-input surfaces are the style-class parser, markdown and
HTML ingest, text shaping, and SVG baking. There are no fuzz targets
yet. Do not feed untrusted documents to those parsers.
