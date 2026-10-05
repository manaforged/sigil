use sigil::prelude::Tone;

use super::data::JobStatus::{Building, Deploying, Failed, Live, Queued, RolledBack};
use super::data::Region::{ApSouth, EuWest, UsEast};
use super::data::Service::{Auth, Billing, Ledger, Media, Search};
use super::data::{Event, Job, LogLine, Step, job};

pub(crate) const CLOCK_START: &str = "10:44:36";
pub(crate) const DURATIONS: [f32; 14] = [
    3.4, 3.1, 3.8, 2.9, 3.3, 4.1, 3.6, 3.2, 2.7, 3.5, 3.9, 3.0, 3.3, 3.5,
];
pub(crate) const SUCCESS_TREND: [f32; 12] = [
    94.0, 95.5, 93.8, 96.0, 97.1, 95.9, 96.4, 97.8, 96.1, 95.2, 96.9, 96.4,
];
pub(crate) const SUCCESS_RATE: &str = "96.4%";
pub(crate) const SUCCESS_LOW: f32 = 90.0;
pub(crate) const SUCCESS_HIGH: f32 = 100.0;
pub(crate) const P95_DURATION: &str = "3m 48s";
pub(crate) const P95_TREND: &str = "-12s vs last week";

pub(crate) static JOBS: [Job; 22] = [
    job(
        Ledger,
        (
            "a41c9e2",
            "ana",
            "Batch ledger writes behind the new outbox",
        ),
        ("main", UsEast, "runner-04"),
        ("10:41:52", 212, 184),
        Deploying,
    ),
    job(
        Ledger,
        ("7f3b0d1", "mo", "Retry idempotent posts on conflict"),
        ("main", EuWest, "runner-07"),
        ("10:43:10", 168, 183),
        Building,
    ),
    job(
        Ledger,
        ("c92e4a7", "lee", "Raise pool size for month-end close"),
        ("release/24.10", UsEast, "runner-02"),
        ("10:44:01", 174, 183),
        Queued,
    ),
    job(
        Ledger,
        ("5d17f60", "rui", "Drop legacy currency rounding path"),
        ("main", ApSouth, "runner-04"),
        ("10:22:37", 196, 182),
        Live,
    ),
    job(
        Ledger,
        ("e08a3b5", "ana", "Add trace ids to settlement events"),
        ("main", UsEast, "runner-01"),
        ("10:05:14", 204, 182),
        Live,
    ),
    job(
        Ledger,
        ("19bc7d4", "mo", "Tighten schema check on ledger import"),
        ("feat/import-check", EuWest, "runner-03"),
        ("09:51:40", 87, 181),
        Failed,
    ),
    job(
        Ledger,
        ("3e6f2a9", "lee", "Cache account lookups for thirty seconds"),
        ("main", UsEast, "runner-02"),
        ("09:30:02", 221, 181),
        Live,
    ),
    job(
        Ledger,
        ("b7d40c8", "rui", "Switch ledger reads to the replica set"),
        ("main", EuWest, "runner-05"),
        ("09:12:47", 245, 180),
        RolledBack,
    ),
    job(
        Ledger,
        ("8a2e91f", "ana", "Split the posting handler into stages"),
        ("main", ApSouth, "runner-06"),
        ("08:58:19", 189, 180),
        Live,
    ),
    job(
        Ledger,
        ("d4f6c03", "mo", "Pin the base image to a fixed digest"),
        ("main", UsEast, "runner-01"),
        ("08:40:55", 176, 179),
        Live,
    ),
    job(
        Search,
        ("f1e2a30", "lee", "Rebuild the shard map on schema change"),
        ("main", EuWest, "runner-08"),
        ("10:42:44", 301, 412),
        Building,
    ),
    job(
        Search,
        (
            "6b9c1d2",
            "rui",
            "Lower the refresh interval to five seconds",
        ),
        ("main", EuWest, "runner-03"),
        ("10:12:09", 288, 410),
        Live,
    ),
    job(
        Search,
        ("0e7a5f4", "ana", "Stream deletes into the tombstone index"),
        ("feat/tombstones", UsEast, "runner-02"),
        ("10:44:20", 295, 411),
        Queued,
    ),
    job(
        Media,
        ("2c8d6e1", "mo", "Add an AV1 ladder for 4K sources"),
        ("main", ApSouth, "runner-06"),
        ("10:44:28", 402, 655),
        Queued,
    ),
    job(
        Media,
        ("94a0b7e", "lee", "Cap encoder threads per pod"),
        ("main", ApSouth, "runner-06"),
        ("09:47:31", 377, 652),
        Live,
    ),
    job(
        Media,
        ("47e3c2d", "rui", "Retry stalled segments once"),
        ("fix/stalls", UsEast, "runner-05"),
        ("09:20:15", 133, 652),
        Failed,
    ),
    job(
        Auth,
        ("5a1f8b3", "ana", "Rotate signing keys every twelve hours"),
        ("main", UsEast, "runner-01"),
        ("10:30:48", 142, 96),
        Live,
    ),
    job(
        Auth,
        ("c3e9d70", "mo", "Rate limit token refresh per client"),
        ("main", EuWest, "runner-07"),
        ("10:44:31", 150, 96),
        Queued,
    ),
    job(
        Auth,
        ("8f2b6a4", "lee", "Shorten session cookie lifetime"),
        ("main", UsEast, "runner-01"),
        ("09:58:03", 139, 95),
        Live,
    ),
    job(
        Billing,
        ("e5d0c19", "rui", "Batch invoice renders overnight"),
        ("main", UsEast, "runner-04"),
        ("10:44:33", 233, 228),
        Queued,
    ),
    job(
        Billing,
        ("1b7f3e8", "ana", "Move retries to the shared queue"),
        ("main", EuWest, "runner-03"),
        ("10:01:26", 219, 227),
        Live,
    ),
    job(
        Billing,
        ("9c4a2d5", "mo", "Round tax per line, not per invoice"),
        ("fix/tax-rounding", UsEast, "runner-02"),
        ("09:39:12", 91, 227),
        Failed,
    ),
];

const fn line(slot: usize, time: &'static str, text: &'static str, tone: Tone) -> LogLine {
    LogLine {
        slot,
        time,
        text,
        tone,
    }
}

pub(crate) const LOG_SEED: [LogLine; 20] = [
    line(0, "10:41:52", "job accepted by runner-04", Tone::Neutral),
    line(0, "10:41:53", "checked out a41c9e2 on main", Tone::Neutral),
    line(0, "10:42:08", "pulled image sha256:4be1d07c", Tone::Neutral),
    line(0, "10:43:31", "build finished in 1m 38s", Tone::Success),
    line(0, "10:43:40", "pushed ledger-api:a41c9e2", Tone::Neutral),
    line(0, "10:44:02", "canary 25% in us-east", Tone::Info),
    line(0, "10:44:19", "health check passed 1/3", Tone::Success),
    line(0, "10:44:35", "health check passed 2/3", Tone::Success),
    line(1, "10:43:10", "job accepted by runner-07", Tone::Neutral),
    line(1, "10:43:12", "checked out 7f3b0d1 on main", Tone::Neutral),
    line(
        1,
        "10:43:29",
        "restored cache, 3 of 4 layers hit",
        Tone::Neutral,
    ),
    line(1, "10:43:58", "compiling ledger-api", Tone::Info),
    line(5, "09:51:40", "job accepted by runner-03", Tone::Neutral),
    line(
        5,
        "09:52:51",
        "schema check failed on import_batches",
        Tone::Error,
    ),
    line(
        5,
        "09:53:07",
        "job failed, nothing was deployed",
        Tone::Error,
    ),
    line(7, "09:12:47", "job accepted by runner-05", Tone::Neutral),
    line(7, "09:15:40", "canary 25% in eu-west", Tone::Info),
    line(7, "09:16:31", "health check failed 2/3", Tone::Error),
    line(7, "09:16:52", "rolled back to 3e6f2a9", Tone::Warn),
    line(3, "10:25:53", "ledger-api 5d17f60 is live", Tone::Success),
];

const fn at(after_ms: u64, event: Event) -> Step {
    Step { after_ms, event }
}

const fn log(slot: usize, time: &'static str, text: &'static str, tone: Tone) -> Event {
    Event::Log(line(slot, time, text, tone))
}

pub(crate) const SCRIPT: [Step; 24] = [
    at(0, Event::Clock("10:44:51")),
    at(
        0,
        log(0, "10:44:51", "health check passed 3/3", Tone::Success),
    ),
    at(900, Event::Clock("10:45:06")),
    at(0, log(0, "10:45:06", "canary 100% in us-east", Tone::Info)),
    at(900, Event::Status(0, Live)),
    at(
        0,
        log(0, "10:45:07", "ledger-api a41c9e2 is live", Tone::Success),
    ),
    at(0, Event::Duration(3.5)),
    at(900, Event::Status(2, Building)),
    at(
        0,
        log(2, "10:45:10", "job accepted by runner-02", Tone::Neutral),
    ),
    at(
        800,
        log(1, "10:45:14", "build finished in 2m 04s", Tone::Success),
    ),
    at(0, Event::Status(1, Deploying)),
    at(1200, Event::Status(12, Building)),
    at(0, Event::Clock("10:45:22")),
    at(
        1500,
        log(1, "10:45:31", "canary 25% in eu-west", Tone::Info),
    ),
    at(
        1400,
        log(1, "10:45:47", "health check passed 1/3", Tone::Success),
    ),
    at(
        1400,
        log(1, "10:46:03", "health check passed 2/3", Tone::Success),
    ),
    at(
        1400,
        log(1, "10:46:19", "health check passed 3/3", Tone::Success),
    ),
    at(0, Event::Clock("10:46:19")),
    at(900, Event::Status(1, Live)),
    at(
        0,
        log(1, "10:46:20", "ledger-api 7f3b0d1 is live", Tone::Success),
    ),
    at(0, Event::Duration(2.8)),
    at(1100, Event::Status(10, Deploying)),
    at(1600, Event::Status(2, Deploying)),
    at(0, Event::Clock("10:46:41")),
];
