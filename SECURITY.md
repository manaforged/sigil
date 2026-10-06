# Security

## Report a vulnerability

Email [thomas@manaforge.tech](mailto:thomas@manaforge.tech). You can also open the repository's **Security** tab and select **Report a vulnerability**. Do not open a public issue.

## Supported versions

The latest `0.x` release gets security fixes. Older `0.x` releases do not.

## Input surfaces

Sigil parses these inputs. Parsing runs on the UI thread unless the entry says otherwise. A large input can stall a frame.

| Input | What the code does |
| --- | --- |
| `markdown` | A line-based parser. A source larger than 1 MiB renders a single "larger than the 1 MiB limit" paragraph. With `image-decode`, an image whose source is a base64 `data:` URI is decoded with the `image` crate and shown at most 480 x 384 px. SVG `data:` URIs are not decoded. Any other image source shows the alt text. Sigil fetches nothing from the network. |
| `html` | A subset parser with no CSS and no scripts. The 1 MiB limit applies. |
| SVG icons | Parsed with `usvg`. The 1 MiB limit applies. A source with element nesting deeper than 128 is rejected before `usvg` runs. External and embedded image references are not resolved. |
| `mermaid` | The 1 MiB limit applies. The parser reads at most 256 lines of at most 512 characters, and at most 1024 flowchart edges. Output is cut to 512 lines of 1024 characters. |
| Class strings (`.class("...")`) | Split on spaces and parsed token by token. An unknown class panics in a debug build. In a release build it logs one warning and has no effect. The parser keeps at most 4096 distinct theme token names for the life of the process. After that, a new `bg-`, `text-`, or `border-` theme name counts as an unknown class. |
| Theme TOML (`parse_theme`, `AppConfig::theme_file`, `watch_theme`) | Parsed with the `toml` crate. `base` must be `light`, `dark`, or `console_dark`. Each `[tokens]` value must be a hex colour. An invalid file is ignored and the current theme stays. `watch_theme` watches the file on a background thread, waits for writes to stop (at most 1 s), then reads and parses the file on the UI thread. There is no file size limit. |
| `open` (feature `desktop`) | Accepts only targets that start with `http://`, `https://`, or `mailto:`, case-insensitive. It rejects every other target, including file paths. |
| PTY output (`Session`, feature `pty`) | A background thread reads the child output in 4 KiB chunks into a `vt100` parser with a 24 x 80 screen and no scrollback. The UI receives the screen as plain text rows. |
| Clipboard (`paste`) | Returns the clipboard text as a `String`. A terminal bracketed paste arrives as one key event. `text_field` ignores a paste that contains a newline. |
| Terminal input | `crossterm` decodes the key, mouse, focus, and paste escape sequences. |

## Fuzzing

The `fuzz/` crate holds six `cargo-fuzz` targets:

| Target | Entry point |
| --- | --- |
| `markdown` | `markdown` widget, built and flushed in a `Cx` |
| `html` | `html` widget, built and flushed in a `Cx` |
| `svg_icon` | `IconBaker::bake` at 24 x 24 |
| `mermaid` | `mermaid` |
| `style_class` | `parse_class_string` and `parse_stateful_token` |
| `scene_render` | `CpuRenderer::render` on generated scenes |

On 2026-09-24 each parser target ran for 1 hour on x86_64 Linux with AddressSanitizer and `-max_len=1048576`. The `markdown` target found two faults, which are fixed:

- Deeply nested quotes and lists overflowed the stack. The parser now flattens nesting deeper than 16 levels into a paragraph.
- The path raster cache held up to 256 entries of any size and grew past 2 GiB. The cache now also holds at most 64 MiB of pixels.

After the fixes, no run found a crash, a leak, or an out-of-memory condition:

| Target | Executions |
| --- | --- |
| `markdown` | 1,377,948 |
| `html` | 5,778,975 |
| `svg_icon` | 58,733,417 |
| `mermaid` | 1,502,693 |
| `style_class` | 20,593,303 |

### Run the fuzz targets

The targets need a nightly toolchain and `cargo-fuzz` on Linux or macOS.

```sh
cargo install cargo-fuzz --locked
cd fuzz
cargo +nightly fuzz run markdown -- -max_total_time=3600 -max_len=1048576
```

A crash input goes to `fuzz/artifacts/<target>/`. Minimize it with `cargo +nightly fuzz tmin <target> <input>`.

## sigil-sign trust model

- `sign_file` and `sign_file_with_key` produce a standard minisign signature over the package bytes. The `.sig` file is the plain minisign envelope. The trusted comment is `file:<name>\tversion:<version>`, and the signature covers it.
- `verify_file` checks the signature against the public key, then checks that the trusted comment names the file. A signature moved to another file name fails.
- Rollback: `verify_file` refuses a package signed for another version when the caller passes `expected_version`. Without it, an older signed package still verifies. The updater must pass the version it expects to install.
- `sigil-pack sign` and `sigil-pack genkey` read the key password from `SIGIL_PACK_PASSWORD` and refuse a `--password` flag. `sign` refuses to write the signature through a symbolic link or over a non-regular file. `genkey` creates the secret key with mode 0600 and never overwrites a file.
- Bundle copies open the source once and check that the opened file is the one that passed the symlink check (device and inode on Unix). Output files are created new; an existing path fails.
