use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use crate::event::Event;
use crate::store;

pub(crate) fn write(path: &Path) -> std::io::Result<()> {
    let events = store::events();
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    let mut out = BufWriter::new(File::create(path)?);
    writeln!(out, "{{\"displayTimeUnit\":\"ms\",\"traceEvents\":[")?;
    write_meta(&mut out, events.is_empty())?;
    for (i, event) in events.iter().enumerate() {
        write_event(&mut out, *event)?;
        if i + 1 < events.len() {
            writeln!(out, ",")?;
        } else {
            writeln!(out)?;
        }
    }
    writeln!(out, "]}}")?;
    out.flush()
}

fn write_meta(out: &mut BufWriter<File>, last: bool) -> std::io::Result<()> {
    writeln!(
        out,
        "{{\"name\":\"process_name\",\"ph\":\"M\",\"pid\":1,\"args\":{{\"name\":\"sigil\"}}}},"
    )?;
    writeln!(
        out,
        "{{\"name\":\"thread_name\",\"ph\":\"M\",\"pid\":1,\"tid\":1,\"args\":{{\"name\":\"ui\"}}}},"
    )?;
    write!(
        out,
        "{{\"name\":\"thread_name\",\"ph\":\"M\",\"pid\":1,\"tid\":2,\"args\":{{\"name\":\"gpu\"}}}}"
    )?;
    if last {
        writeln!(out)
    } else {
        writeln!(out, ",")
    }
}

fn write_event(out: &mut BufWriter<File>, event: Event) -> std::io::Result<()> {
    match event {
        Event::Complete {
            name,
            cat,
            tid,
            start_us,
            dur_us,
        } => write!(
            out,
            "{{\"name\":\"{name}\",\"cat\":\"{cat}\",\"ph\":\"X\",\"ts\":{start_us},\"dur\":{dur_us},\"pid\":1,\"tid\":{tid}}}"
        ),
        Event::Instant { name, cat, ts_us } => write!(
            out,
            "{{\"name\":\"{name}\",\"cat\":\"{cat}\",\"ph\":\"i\",\"s\":\"t\",\"ts\":{ts_us},\"pid\":1,\"tid\":1}}"
        ),
        Event::Counter { name, ts_us, value } => write!(
            out,
            "{{\"name\":\"{name}\",\"ph\":\"C\",\"ts\":{ts_us},\"pid\":1,\"tid\":1,\"args\":{{\"value\":{value}}}}}"
        ),
        Event::FlowStart { name, id, ts_us } => write!(
            out,
            "{{\"name\":\"{name}\",\"cat\":\"input\",\"ph\":\"s\",\"id\":{id},\"ts\":{ts_us},\"pid\":1,\"tid\":1}}"
        ),
        Event::FlowFinish { name, id, ts_us } => write!(
            out,
            "{{\"name\":\"{name}\",\"cat\":\"input\",\"ph\":\"f\",\"bp\":\"e\",\"id\":{id},\"ts\":{ts_us},\"pid\":1,\"tid\":1}}"
        ),
    }
}
