# sigil-sign

App bundles and minisign signing for Sigil release packages.

A release pipeline signs each update package and ships the signature beside
it. This crate writes that signature. The envelope is the reference minisign
format, so any standard minisign reader verifies it.

## Install

```bash
cargo add sigil-sign
```

## Usage

```rust,no_run
let envelope = sigil_sign::sign_file(
    std::path::Path::new("MyApp-0.1.0-full.nupkg"),
    std::path::Path::new("/secure/updater.key"),
    "key password",
)?;
std::fs::write("MyApp-0.1.0-full.nupkg.sig", envelope)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Pass an empty password for an unencrypted key. `sign_file` returns the
envelope as text. The caller decides where to write it. Use the plain
envelope for a standard `.sig` file. Base64-encode it when the transport
requires an ASCII-safe payload.

## License

[MIT](../../LICENSE-MIT). Copyright (c) 2026 Thomas Gardiner.
