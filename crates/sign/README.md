# sigil-sign

App bundles and minisign signing for Sigil release packages.

The crate builds a macOS `.app` bundle or a Windows app directory, and
signs update packages with minisign. The signature is the reference
minisign format, so any standard minisign tool verifies it. The crate
also installs the `sigil-pack` command.

## Install

```bash
cargo add sigil-sign
```

To install the command:

```bash
cargo install sigil-sign
```

## Usage

```rust,no_run
let envelope = sigil_sign::sign_file(
    std::path::Path::new("MyApp-0.1.0-full.nupkg"),
    std::path::Path::new("/secure/updater.key"),
    Some("key password"),
    "0.1.0",
)?;
std::fs::write("MyApp-0.1.0-full.nupkg.sig", envelope)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Pass `None` for an unencrypted key. `sign_file` returns the signature
as text. You decide where to write it. The trusted comment is
`file:<name>\tversion:<version>`.

`sign_file` reads a standard minisign secret key file. If the key
arrives as text, call `sign_file_with_key`. Decode any transport
encoding first; that function takes the key box itself.

`verify_file(public_key, file, sig, expected_version)` checks the
signature, the file name in the trusted comment, and, when given, the
version. It returns the signed version.

`macos_app` and `windows_dir` build the bundles. `sigil-pack macos`,
`sigil-pack windows`, `sigil-pack sign`, and `sigil-pack verify` run the
same steps from the command line. `sigil-pack genkey DIR` writes a new
key pair. `sigil-pack sign` and `sigil-pack genkey` read the key
password from `SIGIL_PACK_PASSWORD`. `sigil-pack sign` writes the plain
minisign envelope to `<file>.sig`; `minisign -V` accepts it.

## License

[MIT](../../LICENSE-MIT). Copyright (c) 2026 Thomas Gardiner.
