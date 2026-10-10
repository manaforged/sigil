# Signing

A Sigil application ships signed release packages. The signer lives in the `sigil-sign` crate. The envelope is the reference minisign format.

## Sign a package

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

`sign_file` reads the package and returns the envelope as text. The caller decides where to write it.

Pass `None` for an unencrypted key. An encrypted key without a password returns `SignError::PasswordRequired`.

The trusted comment is `file:<name>\tversion:<version>`. The signature covers it, so a signature cannot be moved to another file name or version.

## Write the signature

A standard `.sig` file holds the plain envelope. `minisign -Vm <file> -p
<key.pub>` accepts it.

## Verify a package

```rust,no_run
sigil_sign::verify_file(
    std::path::Path::new("updater.pub"),
    std::path::Path::new("MyApp-0.1.0-full.nupkg"),
    std::path::Path::new("MyApp-0.1.0-full.nupkg.sig"),
    "0.1.0",
)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

`verify_file` checks the signature, then checks that the trusted comment names the file and the version you expect to install, so an older signed package is refused. A mismatch returns `SignError::TrustedComment`.

The application embeds the public key. It verifies each package against the signature beside it before it installs the update.

## Embed a macOS provisioning profile

```bash
sigil-pack macos --exe target/release/MyApp --out MyApp.app \
  --name MyApp --id com.example.myapp --version 0.1.0 \
  --identity "Developer ID Application: Example" \
  --provisioning-profile MyApp.provisionprofile \
  --entitlements MyApp.entitlements
```

The builder copies the profile unchanged to `Contents/embedded.provisionprofile` before signing the app. The source must be a regular file, without a symlink. A profile requires a nonempty signing identity. An absent identity or `-` selects ad-hoc signing and is refused when a profile is supplied. You can also supply the identity through `SIGIL_CODESIGN_IDENTITY`.

For the Rust API, set `AppBundle::provisioning_profile` to `Some(path)`. Use `None` for a bundle without a profile. `CodeSign` holds the identity and entitlements. `windows_dir` rejects a profile, and the CLI accepts `--provisioning-profile` only for `macos`.

Supply a profile and entitlements that match your application's identifier and signing identity. The builder does not validate the profile's contents or expiration. Native signing requires macOS. Profile embedding does not notarize the app.
