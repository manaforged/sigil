# Signing

A Sigil application ships signed release packages. The signer lives in the
`sigil-sign` crate. The envelope is the reference minisign format.

## Sign a package

```rust,no_run
let envelope = sigil_sign::sign_file(
    std::path::Path::new("MyApp-0.1.0-full.nupkg"),
    std::path::Path::new("/secure/updater.key"),
    "key password",
)?;
std::fs::write("MyApp-0.1.0-full.nupkg.sig", envelope)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

`sign_file` reads the package and returns the envelope as text. The caller
decides where to write it.

Pass an empty password for an unencrypted key.

## Write the signature

A standard `.sig` file holds the plain envelope. A transport that needs an
ASCII-safe payload base64-encodes the returned string.

The application embeds the public key. It verifies each package against the
signature beside it before it installs the update.
