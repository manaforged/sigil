use super::*;
use crate::events::Event;

fn resolved<T: Clone>(picked: Signal<Status<T, Infallible>>) -> Option<T> {
    picked.with(|status| match status {
        Status::Ok(value) => Some(value.clone()),
        Status::Idle | Status::Pending | Status::Err(_) | Status::Panicked(_) => None,
    })
}

#[test]
fn mock_responses_bypass_the_native_picker() {
    crate::reactive::Owner::scope("mock_responses_bypass_the_native_picker", || {
        mock::reset();
        mock::push_path(Some(PathBuf::from("/tmp/data.csv")));
        assert_eq!(
            resolved(open_file()),
            Some(Some(PathBuf::from("/tmp/data.csv")))
        );
        mock::push_path(None);
        assert_eq!(resolved(save_file()), Some(None));
        mock::push_paths(Some(vec![PathBuf::from("/a"), PathBuf::from("/b")]));
        assert_eq!(
            resolved(FileDialog::new().open_files()),
            Some(Some(vec![PathBuf::from("/a"), PathBuf::from("/b")]))
        );
        mock::reset();
    });
}

#[test]
fn mocked_confirm_does_not_open_a_native_alert() {
    crate::reactive::Owner::scope("mocked_confirm_does_not_open_a_native_alert", || {
        mock::reset();
        mock::push_alert(true);
        assert_eq!(
            resolved(confirm("Clear?", "Remove completed tasks.")),
            Some(true)
        );
        mock::push_alert(false);
        assert_eq!(
            resolved(message("Saved", "All tasks written.")),
            Some(false)
        );
        mock::reset();
    });
}

#[cfg(feature = "terminal")]
#[test]
fn terminal_picker_filters_and_picks_with_the_keyboard() {
    use crate::events::NamedKey;
    use crate::terminal::step;
    let dir = std::env::temp_dir().join(format!("sigil-picker-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("sub")).expect("temp dir");
    std::fs::write(dir.join("a.csv"), "x").expect("csv");
    std::fs::write(dir.join("b.txt"), "x").expect("txt");
    crate::reactive::Owner::scope("terminal_picker", || {
        mock::reset();
        let slot = std::rc::Rc::new(std::cell::RefCell::new(None));
        let writer = std::rc::Rc::clone(&slot);
        let root = dir.clone();
        let mut host = crate::Host::new(crate::kernel::HostMode::Cell);
        host.mount(move || {
            *writer.borrow_mut() = Some(
                FileDialog::new()
                    .directory(&root)
                    .filter("CSV", &["csv"])
                    .open_file(),
            );
            crate::kernel::text("root")
        });
        let listing = step(&mut host, [], 60, 20).expect("frame").text();
        assert!(
            listing.contains("a.csv") && !listing.contains("b.txt"),
            "{listing}"
        );
        let keys = [
            Event::key_down(NamedKey::ArrowDown),
            Event::key_down(NamedKey::ArrowDown),
        ];
        let _ = step(&mut host, keys, 60, 20);
        let _ = step(&mut host, [Event::key_down(NamedKey::Enter)], 60, 20);
        let picked = slot.borrow().and_then(resolved);
        assert_eq!(picked, Some(Some(dir.join("a.csv"))));
    });
    let _ = std::fs::remove_dir_all(&dir);
}
