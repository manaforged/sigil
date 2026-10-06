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
