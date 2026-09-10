use core::fmt;

use sigil::prelude::Tone;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Level {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl Level {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }

    pub(crate) const fn tone(self) -> Tone {
        match self {
            Self::Trace => Tone::Neutral,
            Self::Debug => Tone::Info,
            Self::Info => Tone::Success,
            Self::Warn => Tone::Warn,
            Self::Error => Tone::Error,
        }
    }

    pub(crate) const fn all() -> [Self; 5] {
        [
            Self::Trace,
            Self::Debug,
            Self::Info,
            Self::Warn,
            Self::Error,
        ]
    }

    const fn from_index(index: usize) -> Self {
        match index % 5 {
            0 => Self::Trace,
            1 => Self::Debug,
            2 => Self::Info,
            3 => Self::Warn,
            _ => Self::Error,
        }
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct LogLine {
    pub(crate) timestamp: String,
    pub(crate) level: Level,
    pub(crate) message: String,
}

impl LogLine {
    pub(crate) fn format(&self) -> String {
        format!("{} {:>5} {}", self.timestamp, self.level, self.message)
    }
}

const TOPICS: &[&str] = &[
    "worker",
    "scheduler",
    "ingest",
    "compactor",
    "shell",
    "renderer",
    "audio-thread",
    "fs-watch",
];

const VERBS: &[&str] = &[
    "started",
    "stalled",
    "recovered",
    "flushed buffer",
    "dropped frame",
    "accepted connection",
    "rejected packet",
    "promoted block",
];

fn pick<'a>(table: &'a [&'a str], index: usize) -> &'a str {
    table.get(index % table.len()).copied().unwrap_or("?")
}

pub(crate) fn generate(count: usize) -> Vec<LogLine> {
    let mut lines = Vec::with_capacity(count);
    for index in 0..count {
        let level = Level::from_index(index);
        let topic = pick(TOPICS, index);
        let verb = pick(VERBS, index / 5);
        let seq = index + 1;
        let ms = (index * 37) % 1_000;
        let total_seconds = (index / 4) % 86_400;
        let hours = total_seconds / 3_600;
        let minutes = (total_seconds % 3_600) / 60;
        let seconds = total_seconds % 60;
        let timestamp = format!("{hours:02}:{minutes:02}:{seconds:02}.{ms:03}");
        let message = format!("[{topic}#{seq:05}] {verb}");
        lines.push(LogLine {
            timestamp,
            level,
            message,
        });
    }
    lines
}

#[cfg(test)]
#[path = "data_tests.rs"]
mod tests;
