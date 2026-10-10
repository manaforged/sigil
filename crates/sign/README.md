# sigil-sign

App bundles and minisign signing for Sigil release packages.

The crate builds a macOS `.app` bundle or a Windows app directory, and signs update packages with minisign. The signature is the reference minisign format, so any standard minisign tool verifies it. The crate also installs the `sigil-pack` command.

## Install

```bash
cargo add --git https://github.com/manaforged/sigil sigil-sign
```

To install the command:

```bash
cargo install --git https://github.com/manaforged/sigil sigil-sign
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

Pass `None` for an unencrypted key. `sign_file` returns the signature as text. You decide where to write it. The trusted comment is `file:<name>\tversion:<version>`.

`sign_file` reads a standard minisign secret key file. If the key arrives as text, call `sign_file_with_key`. Decode any transport encoding first; that function takes the key box itself.

`verify_file(public_key, file, sig, expected_version)` checks the signature, and the file name and version in the trusted comment.

`macos_app` and `windows_dir` build the bundles. `sigil-pack macos`, `sigil-pack windows`, `sigil-pack sign`, and `sigil-pack verify` run the same steps from the command line. `sigil-pack genkey DIR` writes a new key pair. `sigil-pack sign` and `sigil-pack genkey` read the key password from `SIGIL_PACK_PASSWORD`. `sigil-pack sign` writes the plain minisign envelope to `<file>.sig`; `minisign -V` accepts it.

Set `AppBundle::provisioning_profile` to `Some(path)` to embed a macOS provisioning profile, or `None` to omit it. The CLI accepts `sigil-pack macos --provisioning-profile PATH`. The bundle builder copies the regular, non-symlink file unchanged to `Contents/embedded.provisionprofile` before signing. A profile requires a nonempty signing identity; ad-hoc signing is refused. `windows_dir` and other CLI commands reject provisioning profiles. `CodeSign` signs the completed bundle or an executable; it does not embed profiles.

## API

<!-- truesight:surface sigil-sign -->

37 public items (5 types, 6 functions, 1 method, 12 fields, 13 variants) at 37 paths; 0 are re-export aliases. 10 trait impl lines, 1 inherent impl line. 1 module, 0 re-export only.

| Module | Items | Types | Functions | Methods | Aliases |
| --- | ---: | ---: | ---: | ---: | ---: |
| `sigil_sign` | 37 | 5 | 6 | 1 | 0 |

<!-- /truesight -->

## License

[MIT](https://github.com/manaforged/sigil/blob/main/LICENSE-MIT). Copyright (c) 2026 Thomas Gardiner.
