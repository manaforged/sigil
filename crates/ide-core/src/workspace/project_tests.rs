use super::*;

#[test]
fn missing_workspace_toml_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let project = Project::load(dir.path()).unwrap();
    assert!(project.is_none(), "absent workspace.toml yields None");
}

#[test]
fn parses_full_workspace_toml() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".sigil-ide")).unwrap();
    let toml = r#"
source_roots = ["src", "crates"]
ignore_patterns = ["target", "*.tmp"]

[language.rust]
server = "rust-analyzer"
extensions = ["rs"]

[build]
check = "cargo check"
test = "cargo test"
"#;
    std::fs::write(dir.path().join(".sigil-ide/workspace.toml"), toml).unwrap();
    let project = Project::load(dir.path()).unwrap().expect("loaded");
    assert_eq!(project.source_roots.len(), 2);
    assert_eq!(project.ignore_patterns, vec!["target", "*.tmp"]);
    let rust = project.languages.get("rust").expect("rust language");
    assert_eq!(rust.server.as_deref(), Some("rust-analyzer"));
    assert_eq!(rust.extensions, vec!["rs"]);
    assert_eq!(
        project.build_commands.get("check").map(String::as_str),
        Some("cargo check"),
    );
}

#[test]
fn malformed_toml_is_invaliddata() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".sigil-ide")).unwrap();
    std::fs::write(
        dir.path().join(".sigil-ide/workspace.toml"),
        "this is not = = toml",
    )
    .unwrap();
    let err = Project::load(dir.path()).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
}
