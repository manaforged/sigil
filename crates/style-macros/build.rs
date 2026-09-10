use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=grammar/structural.toml");
    let grammar =
        std::fs::read_to_string("grammar/structural.toml").expect("read grammar/structural.toml");
    let grammar = grammar
        .parse::<toml::Table>()
        .expect("parse structural grammar");
    let atoms = grammar
        .get("atoms")
        .and_then(toml::Value::as_table)
        .expect("structural grammar has [atoms]");

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR is set by Cargo");
    let dest_path = Path::new(&out_dir).join("structural_table.rs");
    let mut file = BufWriter::new(File::create(dest_path).expect("create structural table"));

    write!(
        &mut file,
        "pub(crate) static STRUCTURAL_TABLE: phf::Map<&'static str, StaticAtom> = "
    )
    .expect("write structural table prefix");
    let mut map = phf_codegen::Map::<&str>::new();
    for key in atoms.keys() {
        map.entry(key, "StaticAtom");
    }
    writeln!(&mut file, "{};", map.build()).expect("write structural table");
}
