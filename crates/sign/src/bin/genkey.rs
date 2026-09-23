use std::fs::OpenOptions;
use std::io::Write;

fn main() {
    let password = std::env::var("SIGIL_PACK_PASSWORD").expect("SIGIL_PACK_PASSWORD");
    let dir = std::env::args().nth(1).expect("usage: genkey <dir>");
    let key_path = format!("{dir}/e2e.key");
    let pub_path = format!("{dir}/e2e.pub");
    if std::path::Path::new(&key_path).exists() || std::path::Path::new(&pub_path).exists() {
        eprintln!("{dir} already holds e2e.key/e2e.pub; refusing to overwrite");
        std::process::exit(1);
    }
    let pair = minisign::KeyPair::generate_encrypted_keypair(Some(password)).expect("keypair");
    write_new(
        &dir,
        "e2e.key",
        &pair.sk.to_box(None).expect("sk box").into_string(),
        0o600,
    );
    write_new(
        &dir,
        "e2e.pub",
        &pair.pk.to_box().expect("pk box").into_string(),
        0o644,
    );
    eprintln!("wrote {dir}/e2e.key + e2e.pub");
}

fn write_new(dir: &str, name: &str, contents: &str, mode: u32) {
    let path = format!("{dir}/{name}");
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, mode);
    #[cfg(not(unix))]
    drop(mode);
    options
        .open(&path)
        .and_then(|mut file| file.write_all(contents.as_bytes()))
        .unwrap_or_else(|e| {
            eprintln!("write {path}: {e}");
            std::process::exit(1);
        });
}
