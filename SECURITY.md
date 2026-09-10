# Security

Report a vulnerability through GitHub's private vulnerability reporting
on this repository (Security tab, "Report a vulnerability"). Do not open
a public issue for it.

The untrusted-input surfaces are the style-class parser, the markdown and
HTML ingest, text shaping, and SVG baking. Fuzz targets do not exist yet;
do not feed untrusted documents to those parsers.
