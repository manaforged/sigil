use std::sync::MutexGuard;

use super::*;

fn recording(backend: Option<BackendKind>) -> MutexGuard<'static, ()> {
    let gate = exclusive();
    enable();
    reset();
    if let Some(kind) = backend {
        begin_frame(BackendStatus {
            kind,
            fallback: FallbackCause::None,
        });
    }
    gate
}

fn last_frame() -> Frame {
    frames().last().cloned().expect("frame")
}

fn dump_to(name: &str) -> String {
    let path = std::env::temp_dir().join(name);
    dump(&path).expect("dump");
    let json = std::fs::read_to_string(&path).expect("read dump");
    drop(std::fs::remove_file(path));
    json
}

#[test]
fn disabled_span_is_silent() {
    let _gate = exclusive();
    disable();
    assert!(!enabled());
    drop(span(Phase::Layout));
    assert!(frames().is_empty());
}

#[test]
fn span_records_when_enabled() {
    let _gate = recording(Some(BackendKind::Cpu));
    {
        let _s = span(Phase::Layout);
        count(Counter::TextMeasure);
    }
    tick_frame();
    let last = last_frame();
    assert!(last.phases.iter().any(|(n, _)| *n == Phase::Layout));
    assert_eq!(last.counter(Counter::TextMeasure), 1);
    disable();
}

#[test]
fn pointer_beats_hover_on_the_same_frame() {
    let _gate = recording(Some(BackendKind::Cpu));
    interact(Cause::Hover);
    interact(Cause::Pointer);
    tick_frame();
    assert_eq!(last_frame().cause, Cause::Pointer);
    disable();
}

#[test]
fn count_add_sums() {
    let _gate = recording(Some(BackendKind::Cpu));
    count_add(Counter::SceneCmds, 12);
    count_add(Counter::SceneCmds, 3);
    tick_frame();
    assert_eq!(last_frame().counter(Counter::SceneCmds), 15);
    disable();
}

#[test]
fn dump_writes_chrome_json() {
    let _gate = recording(Some(BackendKind::Cpu));
    interact(Cause::Pointer);
    {
        let _s = span(Phase::Paint);
        count(Counter::SceneEmit);
    }
    let json = dump_to("sigil-profile-dump-test.json");
    for needle in [
        "\"traceEvents\"",
        "process_name",
        "\"name\":\"paint\"",
        "\"ph\":\"i\"",
        "\"ph\":\"s\"",
        "\"cat\":\"input\"",
    ] {
        assert!(json.contains(needle), "{needle}");
    }
    let text = summary();
    assert!(text.contains("paint") && text.contains("pointer"), "{text}");
    assert!(
        text.lines()
            .any(|line| line.contains("scene.emit") && line.contains('1')),
        "counts print as integers: {text}"
    );
    disable();
}

#[test]
fn dump_closes_open_frame() {
    let _gate = recording(None);
    drop(span(Phase::Effect));
    assert!(frames().is_empty(), "not ticked yet");
    dump_to("sigil-profile-close-test.json");
    assert_eq!(frames().len(), 1, "open frame flushed");
    disable();
}

#[test]
fn app_workload_keys_record_under_their_names() {
    let _gate = recording(None);
    begin_frame(BackendStatus::TERMINAL);
    {
        let _s = span(Phase::AppUpdate);
        count_add(Counter::WorkloadFps, 60);
        count_add(Counter::BuildProfile, 2);
    }
    tick_frame();
    let last = last_frame();
    assert!(last.phases.iter().any(|(n, _)| *n == Phase::AppUpdate));
    assert_eq!(Phase::AppUpdate.category(), Category::App);
    assert_eq!(last.counter(Counter::WorkloadFps), 60);
    assert_eq!(Counter::WorkloadFps.name(), "workload.fps");
    assert_eq!(Counter::BuildProfile.name(), "build.profile");
    disable();
}

#[test]
fn summary_names_who_caused_the_cost_and_the_gpu_passes() {
    let _gate = recording(None);
    let subject = || "Button \"Start\"".to_owned();
    for _ in 0..2 {
        begin_frame(BackendStatus {
            kind: BackendKind::Gpu,
            fallback: FallbackCause::None,
        });
        attribute(Hotspot::LayoutDirtySeed, 1, subject);
        attribute(Hotspot::LayoutDirtySeed, 1, subject);
        gpu_pass("scene paint pass", 2_000_000);
        tick_frame();
    }
    let expected = Attribution {
        hotspot: Hotspot::LayoutDirtySeed,
        subject: subject(),
        count: 2,
    };
    assert_eq!(last_frame().hot, vec![expected], "same subject merges");
    let report = summary();
    disable();
    for needle in [
        "budget 8.33ms",
        "layout.dirty.seed",
        "Button \"Start\" \u{d7}2/frame (2 frames)",
        "gpu passes",
        "scene paint pass",
    ] {
        assert!(report.contains(needle), "{report}");
    }
}
