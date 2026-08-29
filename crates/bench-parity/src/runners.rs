mod construct;
mod quote_table;
mod sigil_runner;

pub use sigil_runner::SigilRunner;

fn nanos(start: std::time::Instant) -> u64 {
    u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX)
}
