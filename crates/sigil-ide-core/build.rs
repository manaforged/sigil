//! Enforce: sigil-ide-core never depends on the sigil widget library.
//! IDE primitives must stay substrate-independent so they're consumable
//! beyond sigil's widget surface.

use std::env;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let cargo_toml = std::path::Path::new(&manifest_dir).join("Cargo.toml");
    let raw = std::fs::read_to_string(&cargo_toml).expect("read Cargo.toml");
    // Strip comments + whitespace before checking — comments mentioning
    // `sigil = ` are allowed (this build script itself describes the rule).
    let stripped: String = raw
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    let banned = stripped.contains("\nsigil = ")
        || stripped.contains("\nsigil-renderer = ")
        || stripped.contains("\nsigil-style = ")
        || stripped.contains("\nsigil-app = ");
    assert!(
        !banned,
        "sigil-ide-core MUST NOT depend on the sigil widget library or its widget-specific crates. \
         IDE primitives stay substrate-independent. Found a banned dep in {}. \
         If you genuinely need sigil-kernel (the reactive primitives), that's allowed — \
         only sigil + sigil-renderer + sigil-style + sigil-app are banned.",
        cargo_toml.display()
    );
    println!("cargo:rerun-if-changed=Cargo.toml");
}
