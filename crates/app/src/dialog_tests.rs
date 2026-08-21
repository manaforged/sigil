use super::*;

#[test]
fn builder_chains_without_opening_a_dialog() {
    // Constructing + configuring must not open UI; only the pick_* calls
    // do. This exercises the whole builder surface headlessly.
    let _dialog = FileDialog::new()
        .filter("CSV", &["csv"])
        .filter("All", &["*"])
        .directory("/tmp")
        .file_name("export.csv");
}

#[test]
fn mock_responses_bypass_the_native_picker() {
    mock::reset();
    // A single-path picker returns the queued response, no native UI.
    mock::push_path(Some(PathBuf::from("/tmp/data.csv")));
    assert_eq!(open_file(), Some(PathBuf::from("/tmp/data.csv")));
    // Responses are consumed in call order across the single-path pickers.
    mock::push_path(None);
    assert_eq!(save_file(), None);
    // The multi-file queue is independent.
    mock::push_paths(Some(vec![PathBuf::from("/a"), PathBuf::from("/b")]));
    assert_eq!(
        FileDialog::new().open_files(),
        Some(vec![PathBuf::from("/a"), PathBuf::from("/b")])
    );
    mock::reset();
}
