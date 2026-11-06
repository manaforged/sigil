# Packing

`sigil-sign` writes the native application layout. Your app copies extra files into that layout, then feeds the result to an updater such as Velopack.

Ad-hoc `codesign --sign -` is not a Developer ID or notarized shipping signature.

## macOS

```sh
sigil-pack macos \
  --exe target/release/myapp \
  --out MyApp.app \
  --name MyApp \
  --id com.example.myapp \
  --version 0.1.0 \
  --icon assets/icon.icns \
  --min 10.15 \
  --resource-from assets/data \
  --resource-to data
```

The command writes `Contents/MacOS`, `Contents/Info.plist`, and `Contents/Resources`. Destination paths must stay inside the bundle. Symlinks in the source tree are refused.

### Native code signing

To sign an existing executable or `.app` bundle on macOS:

```sh
sigil-pack codesign \
  --file target/release/myapp \
  --identity 'Developer ID Application: Example Company (TEAMID)' \
  --entitlements entitlements.plist
```

`macos` and `codesign` use the same signer. `--identity` takes precedence over `SIGIL_CODESIGN_IDENTITY`. Without an identity, the signer creates an ad-hoc signature. With an identity, it enables the hardened runtime and requests a secure timestamp. `--entitlements` applies only when an identity is supplied.

The Rust API is `CodeSign::sign(&self, path: &Path) -> Result<(), PackError>` from `sigil_sign`. It signs the target in place and refuses symlinks. Native signing returns an error on other platforms. The `macos` command can still create an unsigned bundle on those platforms.

Signing does not submit the target for notarization or staple a notarization ticket. Use `sigil-pack notarize` before distributing a downloaded macOS app.

### Disk image

```sh
sigil-pack dmg \
  --app MyApp.app \
  --out MyApp.dmg \
  --identity 'Developer ID Application: Example Company (TEAMID)' \
  --background assets/dmg-background.tiff \
  --layout assets/dmg-layout.DS_Store \
  --icon assets/icon.icns
```

The command writes a zlib-compressed image with the app and an `Applications` link, then signs the image under the same identity rules as `codesign`. `--volume` names the mounted disk and defaults to the app's name. `--icon` sets the volume icon. `--background` copies the file to `.background.EXT` at the disk root. `--layout` copies a `.DS_Store` file to the disk root; it holds the window size, the icon positions and the background path, so its volume name and background path must match. Make the layout file once with `dmgbuild`, which uses the same background path, copy `.DS_Store` from the image it builds, and keep it with your assets. The command runs headless and never starts Finder. It refuses to run while a disk with the same name is mounted.

### Notarization

Store a notary credential once per machine:

```sh
xcrun notarytool store-credentials MyProfile --key AuthKey.p8 --key-id KEYID --issuer ISSUER
```

Then submit the signed image and staple the ticket:

```sh
sigil-pack notarize --file MyApp.dmg --profile MyProfile
```

`--profile` takes precedence over `SIGIL_NOTARY_PROFILE`. The command waits for Apple's result, and it fails when no ticket can be stapled.

## Windows

```sh
sigil-pack windows \
  --exe target/release/myapp.exe \
  --out appdir \
  --name myapp.exe \
  --resource-from assets/data \
  --resource-to data
```

## Sign

```sh
SIGIL_PACK_PASSWORD='secret' sigil-pack sign --key updater.key --file package.nupkg --version 0.1.0
```

That writes `package.nupkg.sig` as a plain minisign envelope with the trusted comment `file:package.nupkg\tversion:0.1.0`. `minisign -V` accepts it. Do not put the password on the command line. See [Signing](signing.md).

## Verify

```sh
sigil-pack verify --pub updater.pub --file package.nupkg --version 0.1.0
```

`--sig` defaults to `<file>.sig`. `--version` is required: the command checks the signature, the file name, and the version, so an older signed package is refused.

## Generate a key

```sh
SIGIL_PACK_PASSWORD='secret' sigil-pack genkey keys
```

`sigil-pack genkey DIR` writes `e2e.key` (mode 0600) and `e2e.pub` in `DIR`. It refuses to overwrite existing files.
