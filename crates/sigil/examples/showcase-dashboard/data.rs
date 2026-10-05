use sigil::prelude::{Token, Tone};
use sigil::style::space;

pub(crate) const RAIL_WIDTH: f32 = space(40);
pub(crate) const GROUP_WIDTH: f32 = space(50);
pub(crate) const INSPECTOR_WIDTH: f32 = space(70);
pub(crate) const INSPECTOR_BREAKPOINT: f32 = 1100.0;
pub(crate) const COMMIT_MIN_WIDTH: f32 = space(30);
pub(crate) const BRANCH_WIDTH: f32 = space(22);
pub(crate) const REGION_WIDTH: f32 = space(22);
pub(crate) const DURATION_WIDTH: f32 = space(22);
pub(crate) const STATUS_WIDTH: f32 = space(26);
pub(crate) const SHA_WIDTH: f32 = space(15);
pub(crate) const LOG_HEIGHT: f32 = space(44);
pub(crate) const TEXT_SMALL: f32 = 12.0;
pub(crate) const TEXT_TITLE: f32 = 18.0;
pub(crate) const TEXT_KPI: f32 = 20.0;
pub(crate) const WEIGHT_STRONG: u16 = 600;
pub(crate) const START_DELAY_MS: u64 = 1600;
pub(crate) const DEPLOYS_BEFORE_OPEN: usize = 37;
pub(crate) const CANARY_START: f32 = 25.0;
pub(crate) const CANARY_STEP: f32 = 5.0;
pub(crate) const SURGE_START: i32 = 2;
pub(crate) const SURGE_MAX: i32 = 10;
pub(crate) const HEALTH_PATH: &str = "/healthz";
pub(crate) const VERSION_LINE: &str = "fleet 2.4.1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Section {
    Overview,
    Deploys,
    Services,
    Runners,
    Incidents,
    Audit,
    Settings,
}

impl Section {
    pub(crate) const ALL: [Self; 7] = [
        Self::Overview,
        Self::Deploys,
        Self::Services,
        Self::Runners,
        Self::Incidents,
        Self::Audit,
        Self::Settings,
    ];

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Deploys => "Deploys",
            Self::Services => "Services",
            Self::Runners => "Runners",
            Self::Incidents => "Incidents",
            Self::Audit => "Audit log",
            Self::Settings => "Settings",
        }
    }

    pub(crate) const fn glyph(self) -> &'static str {
        match self {
            Self::Overview => "◆",
            Self::Deploys => "▲",
            Self::Services => "■",
            Self::Runners => "●",
            Self::Incidents => "◎",
            Self::Audit => "≡",
            Self::Settings => "◇",
        }
    }

    pub(crate) const fn badge(self) -> Option<&'static str> {
        match self {
            Self::Runners => Some("8"),
            Self::Incidents => Some("2"),
            Self::Overview | Self::Deploys | Self::Services | Self::Audit | Self::Settings => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Env {
    Production,
    Staging,
    Preview,
}

impl Env {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Production => "production",
            Self::Staging => "staging",
            Self::Preview => "preview",
        }
    }

    pub(crate) const fn tone(self) -> Tone {
        match self {
            Self::Production => Tone::Primary,
            Self::Staging => Tone::Warn,
            Self::Preview => Tone::Info,
        }
    }
}

pub(crate) const ENV_FILTER_LABELS: [&str; 4] =
    ["All environments", "production", "staging", "preview"];
pub(crate) const ENV_FILTERS: [Option<Env>; 4] = [
    None,
    Some(Env::Production),
    Some(Env::Staging),
    Some(Env::Preview),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Bucket {
    Running,
    Queued,
    Succeeded,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Service {
    Ledger,
    Search,
    Media,
    Auth,
    Billing,
}

impl Service {
    pub(crate) const ALL: [Self; 5] = [
        Self::Ledger,
        Self::Search,
        Self::Media,
        Self::Auth,
        Self::Billing,
    ];

    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Ledger => "ledger-api",
            Self::Search => "search-indexer",
            Self::Media => "media-encoder",
            Self::Auth => "auth-gateway",
            Self::Billing => "billing-worker",
        }
    }

    pub(crate) const fn env(self) -> Env {
        match self {
            Self::Ledger | Self::Auth => Env::Production,
            Self::Search | Self::Billing => Env::Staging,
            Self::Media => Env::Preview,
        }
    }

    pub(crate) const fn history(self, bucket: Bucket) -> usize {
        match (self, bucket) {
            (Self::Ledger, Bucket::Succeeded) => 14,
            (Self::Search, Bucket::Succeeded) => 9,
            (Self::Media, Bucket::Succeeded) => 6,
            (Self::Auth, Bucket::Succeeded) => 11,
            (Self::Billing, Bucket::Succeeded) => 4,
            (Self::Ledger | Self::Media, Bucket::Failed) => 1,
            (Self::Billing, Bucket::Failed) => 2,
            (Self::Search | Self::Auth, Bucket::Failed) | (_, Bucket::Running | Bucket::Queued) => {
                0
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Region {
    UsEast,
    EuWest,
    ApSouth,
}

impl Region {
    pub(crate) const ALL: [Self; 3] = [Self::UsEast, Self::EuWest, Self::ApSouth];

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::UsEast => "us-east",
            Self::EuWest => "eu-west",
            Self::ApSouth => "ap-south",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum JobStatus {
    Queued,
    Building,
    Deploying,
    Live,
    Failed,
    RolledBack,
}

impl JobStatus {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Queued => "Queued",
            Self::Building => "Building",
            Self::Deploying => "Deploying",
            Self::Live => "Live",
            Self::Failed => "Failed",
            Self::RolledBack => "Rolled back",
        }
    }

    pub(crate) const fn tone(self) -> Tone {
        match self {
            Self::Queued => Tone::Neutral,
            Self::Building => Tone::Info,
            Self::Deploying => Tone::Primary,
            Self::Live => Tone::Success,
            Self::Failed => Tone::Error,
            Self::RolledBack => Tone::Warn,
        }
    }

    pub(crate) const fn bucket(self) -> Bucket {
        match self {
            Self::Queued => Bucket::Queued,
            Self::Building | Self::Deploying => Bucket::Running,
            Self::Live => Bucket::Succeeded,
            Self::Failed | Self::RolledBack => Bucket::Failed,
        }
    }

    pub(crate) const fn progress(self) -> f32 {
        match self {
            Self::Queued => 0.0,
            Self::Building => 0.35,
            Self::Deploying => 0.7,
            Self::Failed => 0.5,
            Self::Live | Self::RolledBack => 1.0,
        }
    }
}

pub(crate) const fn tone_token(tone: Tone) -> Token {
    match tone {
        Tone::Success => Token::Success11,
        Tone::Error => Token::Error11,
        Tone::Warn => Token::Warn11,
        Tone::Info => Token::Info11,
        Tone::Primary => Token::Accent11,
        _ => Token::OnSurface,
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Job {
    pub(crate) service: Service,
    pub(crate) sha: &'static str,
    pub(crate) author: &'static str,
    pub(crate) message: &'static str,
    pub(crate) branch: &'static str,
    pub(crate) region: Region,
    pub(crate) runner: &'static str,
    pub(crate) started: &'static str,
    pub(crate) secs: u32,
    pub(crate) image_mb: u32,
    pub(crate) status: JobStatus,
}

pub(crate) type Commit = (&'static str, &'static str, &'static str);
pub(crate) type Place = (&'static str, Region, &'static str);
pub(crate) type Timing = (&'static str, u32, u32);

pub(crate) const fn job(
    service: Service,
    commit: Commit,
    place: Place,
    timing: Timing,
    status: JobStatus,
) -> Job {
    Job {
        service,
        sha: commit.0,
        author: commit.1,
        message: commit.2,
        branch: place.0,
        region: place.1,
        runner: place.2,
        started: timing.0,
        secs: timing.1,
        image_mb: timing.2,
        status,
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct LogLine {
    pub(crate) slot: usize,
    pub(crate) time: &'static str,
    pub(crate) text: &'static str,
    pub(crate) tone: Tone,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Event {
    Status(usize, JobStatus),
    Log(LogLine),
    Clock(&'static str),
    Duration(f32),
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Step {
    pub(crate) after_ms: u64,
    pub(crate) event: Event,
}

pub(crate) fn duration_label(secs: u32) -> String {
    format!("{}m {:02}s", secs / 60, secs % 60)
}
