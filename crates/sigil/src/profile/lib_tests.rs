use std::sync::MutexGuard;

use super::*;

fn lock() -> MutexGuard<'static, ()> {
    exclusive()
}

#[test]
fn disabled_span_is_silent() {
    let _gate = lock();
    disable();
    assert!(!enabled());
    {
        let _s = span(crate::profile::Phase::Layout);
    }
    assert!(frames().is_empty());
}

#[test]
fn span_records_when_enabled() {
    let _gate = lock();
    enable();
    reset();
    begin_frame(BackendStatus {
        kind: BackendKind::Cpu,
        fallback: FallbackCause::None,
    });
    {
        let _s = span(crate::profile::Phase::Layout);
        count(crate::profile::Counter::TextMeasure);
    }
    tick_frame();
    let got = frames();
    let last = got.last().expect("frame");
    assert!(
        last.phases.iter().any(|(n, _)| *n == Phase::Layout),
        "layout phase present"
    );
    assert_eq!(last.counter(crate::profile::Counter::TextMeasure), 1);
    disable();
}

#[test]
fn pointer_beats_hover_on_the_same_frame() {
    let _gate = lock();
    enable();
    reset();
    begin_frame(BackendStatus {
        kind: BackendKind::Cpu,
        fallback: FallbackCause::None,
    });
    interact(Cause::Hover);
    interact(Cause::Pointer);
    tick_frame();
    let got = frames();
    let last = got.last().expect("frame");
    assert_eq!(last.cause, Cause::Pointer);
    disable();
}

#[test]
fn count_add_sums() {
    let _gate = lock();
    enable();
    reset();
    begin_frame(BackendStatus {
        kind: BackendKind::Cpu,
        fallback: FallbackCause::None,
    });
    count_add(crate::profile::Counter::SceneCmds, 12);
    count_add(crate::profile::Counter::SceneCmds, 3);
    tick_frame();
    assert_eq!(
        frames()
            .last()
            .expect("frame")
            .counter(crate::profile::Counter::SceneCmds),
        15
    );
    disable();
}

#[test]
fn dump_writes_chrome_json() {
    let _gate = lock();
    enable();
    reset();
    begin_frame(BackendStatus {
        kind: BackendKind::Cpu,
        fallback: FallbackCause::None,
    });
    interact(Cause::Pointer);
    {
        let _s = span(crate::profile::Phase::Paint);
        count(crate::profile::Counter::SceneEmit);
    }
    let path = std::env::temp_dir().join("sigil-profile-dump-test.json");
    dump(&path).expect("dump");
    let json = std::fs::read_to_string(&path).expect("read dump");
    assert!(json.contains("\"traceEvents\""), "chrome envelope");
    assert!(json.contains("process_name"), "metadata");
    assert!(json.contains("\"name\":\"paint\""), "span name");
    assert!(json.contains("\"ph\":\"i\""), "input instant");
    assert!(json.contains("\"ph\":\"s\""), "flow start");
    assert!(json.contains("\"cat\":\"input\""), "input category");
    let text = summary();
    assert!(text.contains("paint"), "summary names paint");
    assert!(text.contains("pointer"), "summary names cause");
    assert!(
        text.lines()
            .any(|line| line.contains("scene.emit") && line.contains('1')),
        "counts print as integers: {text}"
    );
    disable();
    drop(std::fs::remove_file(path));
}

#[test]
fn dump_closes_open_frame() {
    let _gate = lock();
    enable();
    reset();
    {
        let _s = span(crate::profile::Phase::Effect);
    }
    assert!(frames().is_empty(), "not ticked yet");
    let path = std::env::temp_dir().join("sigil-profile-close-test.json");
    dump(&path).expect("dump");
    assert_eq!(frames().len(), 1, "open frame flushed");
    disable();
    drop(std::fs::remove_file(path));
}

#[test]
fn app_workload_keys_record_under_their_names() {
    let _gate = lock();
    enable();
    reset();
    begin_frame(BackendStatus::TERMINAL);
    {
        let _s = span(Phase::AppUpdate);
        count_add(Counter::WorkloadFps, 60);
        count_add(Counter::BuildProfile, 2);
    }
    tick_frame();
    let got = frames();
    let last = got.last().expect("frame");
    assert!(last.phases.iter().any(|(n, _)| *n == Phase::AppUpdate));
    assert_eq!(Phase::AppUpdate.category(), Category::App);
    assert_eq!(last.counter(Counter::WorkloadFps), 60);
    assert_eq!(Counter::WorkloadFps.name(), "workload.fps");
    assert_eq!(Counter::BuildProfile.name(), "build.profile");
    disable();
}

#[test]
fn summary_names_who_caused_the_cost_and_the_gpu_passes() {
    let _gate = lock();
    enable();
    reset();
    for _ in 0..2 {
        begin_frame(BackendStatus {
            kind: BackendKind::Gpu,
            fallback: FallbackCause::None,
        });
        attribute(Hotspot::LayoutDirtySeed, 1, || {
            "Button \"Start\"".to_owned()
        });
        attribute(Hotspot::LayoutDirtySeed, 1, || {
            "Button \"Start\"".to_owned()
        });
        gpu_pass("scene paint pass", 2_000_000);
        tick_frame();
    }
    let last = frames().last().cloned().expect("frame");
    assert_eq!(
        last.hot,
        vec![Attribution {
            hotspot: Hotspot::LayoutDirtySeed,
            subject: "Button \"Start\"".to_owned(),
            count: 2,
        }],
        "the same subject merges within a frame"
    );
    let report = summary();
    disable();
    assert!(report.contains("budget 8.33ms"), "{report}");
    assert!(
        report.contains("layout.dirty.seed")
            && report.contains("Button \"Start\" \u{d7}2/frame (2 frames)"),
        "{report}"
    );
    assert!(
        report.contains("gpu passes") && report.contains("scene paint pass"),
        "{report}"
    );
}
