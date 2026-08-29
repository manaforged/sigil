//! Mock file inventory used by the file-browser example.
//!
//! Everything here is invented at compile time — the app never touches
//! the real filesystem so the example is deterministic across machines
//! and works as a `cargo test` smoke target.

use std::fmt::Write as _;

/// A mock filesystem entry with enough surface to populate every tab
/// (Preview, Metadata, Hex) on the right pane.
#[derive(Clone)]
pub struct MockFile {
    /// Display path shown in the left pane.
    pub path: &'static str,
    /// File size in bytes (advertised in Metadata + drives Hex length).
    pub size: usize,
    /// MIME-ish description.
    pub kind: &'static str,
    /// Mtime stamp (string — purely cosmetic).
    pub modified: &'static str,
    /// Body shown in the Preview tab.
    pub preview: &'static str,
}

/// Return the static list of mock files. Order is stable.
#[must_use]
pub fn inventory() -> Vec<MockFile> {
    let mut out = home_entries();
    out.extend(system_entries());
    out
}

fn home_entries() -> Vec<MockFile> {
    vec![
        MockFile {
            path: "/home/user/notes/sigil-roadmap.md",
            size: 4_217,
            kind: "text/markdown",
            modified: "2026-05-20 09:14",
            preview: "# Sigil roadmap\n\n- splitter ergonomics pass\n- virtual_list selection wiring\n- tabs content routing\n- write the dogfood examples\n",
        },
        MockFile {
            path: "/home/user/projects/atlas/Cargo.toml",
            size: 1_829,
            kind: "application/toml",
            modified: "2026-05-19 22:01",
            preview: "[workspace]\nresolver = \"3\"\nmembers = [\"crates/sim\", \"crates/render\", \"crates/world\"]\n",
        },
        MockFile {
            path: "/home/user/projects/atlas/src/main.rs",
            size: 612,
            kind: "text/rust",
            modified: "2026-05-19 22:01",
            preview: "fn main() {\n    atlas::run();\n}\n",
        },
        MockFile {
            path: "/home/user/photos/2026-05-19-sunset.jpg",
            size: 1_984_213,
            kind: "image/jpeg",
            modified: "2026-05-19 20:42",
            preview: "(binary preview unavailable — switch to the Hex tab)",
        },
        MockFile {
            path: "/home/user/photos/2026-05-19-clouds.jpg",
            size: 2_104_882,
            kind: "image/jpeg",
            modified: "2026-05-19 20:43",
            preview: "(binary preview unavailable — switch to the Hex tab)",
        },
        MockFile {
            path: "/home/demo/music/theme.flac",
            size: 28_104_293,
            kind: "audio/flac",
            modified: "2026-05-18 14:00",
            preview: "(binary preview unavailable — switch to the Hex tab)",
        },
    ]
}

fn system_entries() -> Vec<MockFile> {
    vec![
        MockFile {
            path: "/home/user/.bashrc",
            size: 1_204,
            kind: "text/plain",
            modified: "2026-05-15 08:00",
            preview: "# bashrc\nexport PATH=\"$HOME/.cargo/bin:$PATH\"\nalias gs='git status'\n",
        },
        MockFile {
            path: "/home/user/.config/nvim/init.lua",
            size: 8_492,
            kind: "text/lua",
            modified: "2026-05-12 19:23",
            preview: "vim.opt.number = true\nvim.opt.relativenumber = true\n",
        },
        MockFile {
            path: "/etc/hosts",
            size: 312,
            kind: "text/plain",
            modified: "2026-04-02 10:00",
            preview: "127.0.0.1 localhost\n::1 localhost\n",
        },
        MockFile {
            path: "/var/log/sigil/dogfood.log",
            size: 64_512,
            kind: "text/plain",
            modified: "2026-05-20 11:47",
            preview: "[INFO] dogfood session opened\n[INFO] left pane: 12 items\n[INFO] right pane: tabs (Preview/Metadata/Hex)\n",
        },
        MockFile {
            path: "/usr/local/bin/cargo",
            size: 9_204_182,
            kind: "application/x-executable",
            modified: "2026-05-10 03:14",
            preview: "(binary preview unavailable — switch to the Hex tab)",
        },
        MockFile {
            path: "/tmp/scratch.txt",
            size: 17,
            kind: "text/plain",
            modified: "2026-05-20 11:48",
            preview: "remember the milk\n",
        },
    ]
}

/// Render an ASCII hex view of the first `limit` synthetic bytes derived
/// from `seed`. Output shape: `OFFSET  HH HH HH ... HH  |ASCII|` per line.
#[must_use]
pub fn hex_dump(seed: usize, limit: usize) -> String {
    const WIDTH: usize = 16;
    let total = limit.min(256);
    let mut out = String::with_capacity(total * 4);
    let mut offset = 0;
    while offset < total {
        let _ = write!(out, "{offset:08x}  ");
        let end = (offset + WIDTH).min(total);
        write_hex_bytes(&mut out, seed, offset, end);
        for _ in end..(offset + WIDTH) {
            out.push_str("   ");
        }
        out.push_str(" |");
        write_ascii_bytes(&mut out, seed, offset, end);
        out.push_str("|\n");
        offset += WIDTH;
    }
    out
}

fn write_hex_bytes(out: &mut String, seed: usize, start: usize, end: usize) {
    for byte_index in start..end {
        let byte = synth_byte(seed, byte_index);
        let _ = write!(out, "{byte:02x} ");
    }
}

fn write_ascii_bytes(out: &mut String, seed: usize, start: usize, end: usize) {
    for byte_index in start..end {
        let byte = synth_byte(seed, byte_index);
        let ch = if (0x20..0x7f).contains(&byte) {
            byte as char
        } else {
            '.'
        };
        out.push(ch);
    }
}

const fn synth_byte(seed: usize, index: usize) -> u8 {
    // Pure deterministic mix — no hashing crate dependency needed.
    let mixed = seed
        .wrapping_mul(2_654_435_761)
        .wrapping_add(index.wrapping_mul(40_503));
    #[allow(
        clippy::cast_possible_truncation,
        reason = "values are bounded to the target range before the cast"
    )]
    let truncated = (mixed >> 16) as u8;
    truncated
}
