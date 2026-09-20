# Packing

`sigil-sign` writes the native application layout. The host app (and any
other Sigil app) copies extra site files into that layout, then feeds
the result to Velopack or another updater.

## macOS

```sh
sigil-pack macos \
  --exe target/release/app \
  --out App.app \
  --name App \
  --id com.example.app \
  --version 0.5.0 \
  --icon src-tauri/icons/icon.icns \
  --min 10.15 \
  --resource crates/hcaptcha/models:_up_/crates/hcaptcha/models
```

The command writes `Contents/MacOS`, `Contents/Info.plist`, and
`Contents/Resources`. On macOS it runs `codesign --sign -`.

## Windows

```sh
sigil-pack windows \
  --exe target/release/app.exe \
  --out appdir \
  --name app.exe \
  --resource crates/hcaptcha/models:_up_/crates/hcaptcha/models
```

## Sign

```sh
sigil-pack sign --key updater.key --password secret --file package.nupkg
```

That writes `package.nupkg.sig` as the plain minisign envelope. A feed
that needs ASCII may base64-encode the file. See [Signing](signing.md).
