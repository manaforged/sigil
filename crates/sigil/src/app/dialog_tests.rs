use super::*;

#[test]
fn builder_chains_without_opening_a_dialog() {
    let _dialog = FileDialog::new()
        .filter("CSV", &["csv"])
        .filter("All", &["*"])
        .directory("/tmp")
        .file_name("export.csv");
}

#[test]
fn mock_responses_bypass_the_native_picker() {
    mock::reset();
    mock::push_path(Some(PathBuf::from("/tmp/data.csv")));
    assert_eq!(open_file(), Some(PathBuf::from("/tmp/data.csv")));
    mock::push_path(None);
    assert_eq!(save_file(), None);
    mock::push_paths(Some(vec![PathBuf::from("/a"), PathBuf::from("/b")]));
    assert_eq!(
        FileDialog::new().open_files(),
        Some(vec![PathBuf::from("/a"), PathBuf::from("/b")])
    );
    mock::reset();
}

#[test]
fn mocked_confirm_does_not_open_a_native_alert() {
    mock::reset();
    mock::push_alert(true);
    assert!(confirm("Clear?", "Remove completed tasks."));
    mock::push_alert(false);
    assert!(!message("Saved", "All tasks written."));
    mock::reset();
}
