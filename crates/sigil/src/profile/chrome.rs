use std::fs::{File, create_dir_all};
use std::io::{self, BufWriter, Write};
use std::path::Path;

use serde_json::{Value, json, to_writer};

use crate::profile::event::Event;
use crate::profile::key::{Counter, Phase};
use crate::profile::store::{self, Frame};

pub(crate) fn write(path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        create_dir_all(parent)?;
    }
    let mut out = BufWriter::new(File::create(path)?);
    let frames = store::frames();
    write!(out, "{{\"displayTimeUnit\":\"ms\",\"metadata\":")?;
    value(&mut out, &store::metadata())?;
    write!(out, ",\"frames\":[")?;
    list(&mut out, frames.iter().map(frame_value))?;
    write!(out, "],\"traceEvents\":[")?;
    let names = [
        json!({
            "name": "process_name", "ph": "M", "pid": 1,
            "args": {"name": "sigil"}
        }),
        json!({
            "name": "thread_name", "ph": "M", "pid": 1, "tid": 1,
            "args": {"name": "UI CPU (includes render submission)"}
        }),
    ];
    let attributions = frames.iter().flat_map(|frame| {
        frame.hot.iter().map(|hot| {
            json!({
                "name": hot.hotspot.name(), "cat": "why", "ph": "i", "s": "t",
                "ts": frame.end_us, "pid": 1, "tid": 1,
                "args": {"hotspot": hot.hotspot.name(), "subject": hot.subject, "count": hot.count}
            })
        })
    });
    list(
        &mut out,
        names
            .into_iter()
            .chain(store::events().into_iter().map(event_value))
            .chain(attributions),
    )?;
    writeln!(out, "]}}")?;
    out.flush()
}

fn frame_value(frame: &Frame) -> Value {
    json!({
        "index": frame.index, "cause": frame.cause.name(),
        "wall_ns": frame.total_ns(),
        "phases_ns": named(&frame.phases, Phase::name),
        "counts": named(&frame.counts, Counter::name),
        "gpu_passes_ns": frame.gpu_passes,
        "backend": frame.backend.map(|status| status.to_string())
    })
}

fn list(out: &mut BufWriter<File>, values: impl Iterator<Item = Value>) -> io::Result<()> {
    for (index, item) in values.enumerate() {
        if index > 0 {
            write!(out, ",")?;
        }
        value(out, &item)?;
    }
    Ok(())
}

fn value(out: &mut BufWriter<File>, value: &Value) -> io::Result<()> {
    to_writer(out, value).map_err(io::Error::other)
}

fn event_value(event: Event) -> Value {
    match event {
        Event::Complete {
            phase,
            tid,
            start_us,
            dur_us,
        } => json!({
            "name": phase.name(), "cat": phase.category().name(), "ph": "X", "ts": start_us,
            "dur": dur_us, "pid": 1, "tid": tid
        }),
        Event::Instant { name, cat, ts_us } => json!({
            "name": name, "cat": cat, "ph": "i", "s": "t",
            "ts": ts_us, "pid": 1, "tid": 1
        }),
        Event::Counter {
            counter,
            ts_us,
            value,
        } => json!({
            "name": counter.name(), "ph": "C", "ts": ts_us,
            "pid": 1, "tid": 1, "args": {"value": value}
        }),
        Event::FlowStart { name, id, ts_us } => json!({
            "name": name, "cat": "input", "ph": "s", "id": id,
            "ts": ts_us, "pid": 1, "tid": 1
        }),
        Event::FlowFinish { name, id, ts_us } => json!({
            "name": name, "cat": "input", "ph": "f", "bp": "e",
            "id": id, "ts": ts_us, "pid": 1, "tid": 1
        }),
    }
}

fn named<K: Copy>(list: &[(K, u64)], name: fn(K) -> &'static str) -> Vec<(&'static str, u64)> {
    list.iter()
        .map(|&(key, value)| (name(key), value))
        .collect()
}
