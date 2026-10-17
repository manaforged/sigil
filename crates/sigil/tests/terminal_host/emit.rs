use sigil::Host;
use sigil::kernel::{container, text};
use sigil::layout::LayoutStyle;
use sigil::prelude::LayoutBuilderExt;
use sigil::profile::{self, BackendStatus, Counter};
use sigil::reactive::{Runtime, signal};
use sigil::scene::CommandScene;

const VIEW: (f32, f32) = (320.0, 160.0);

fn counted(work: impl FnOnce()) -> (u64, u64) {
    profile::enable();
    profile::reset();
    profile::begin_frame(BackendStatus::TERMINAL);
    work();
    profile::tick_frame();
    let frame = profile::frames().last().cloned();
    profile::disable();
    frame.map_or((0, 0), |frame| {
        (
            frame.counter(Counter::EmitNodes),
            frame.counter(Counter::EmitCopied),
        )
    })
}

#[test]
fn an_unchanged_subtree_is_copied() {
    Runtime::scope("emit-copy", || {
        let word = signal(String::from("one"));
        let mut host = Host::cells();
        host.mount(move || {
            container()
                .layout(LayoutStyle::flex_column().fill_width())
                .child((0..12).fold(
                    container().layout(LayoutStyle::flex_column()),
                    |column, row| column.child(text(format!("steady row {row}"))),
                ))
                .child(text(word))
        });
        host.layout(VIEW).expect("layout");
        let delta = host.cx().take_delta();
        let (first, _) = host.emit_since(VIEW, &delta, &CommandScene::default());
        word.set(String::from("two"));
        host.layout(VIEW).expect("layout");
        let delta = host.cx().take_delta();
        let mut second = CommandScene::default();
        let (walked, copied) = counted(|| {
            let (scene, diff) = host.emit_since(VIEW, &delta, &first);
            assert!(!diff.unchanged());
            second = scene;
        });
        assert_eq!(
            second,
            host.emit(VIEW),
            "the incremental emit differs from a fresh one"
        );
        assert!(copied > 0, "nothing was copied (walked {walked})");
        host.layout(VIEW).expect("layout");
        let delta = host.cx().take_delta();
        let (idle_walked, _) = counted(|| {
            let (third, diff) = host.emit_since(VIEW, &delta, &second);
            assert!(diff.unchanged());
            assert_eq!(third, second);
        });
        assert_eq!(idle_walked, 0, "an idle frame walked {idle_walked} nodes");
    });
}
