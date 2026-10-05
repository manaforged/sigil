use std::time::Duration;

use sigil::prelude::{
    ScheduleError, Signal, SignalWriter, Status, ToastQueue, signal, signal_writer, spawn,
};
use sigil::tasks::sleep;

use super::data::{
    Bucket, CANARY_START, Event, HEALTH_PATH, Job, JobStatus, LogLine, START_DELAY_MS, SURGE_START,
    Section, Service,
};
use super::seed::{CLOCK_START, DURATIONS, JOBS, LOG_SEED, SCRIPT};

#[derive(Clone, Copy)]
pub(crate) struct Console {
    pub(crate) statuses: Signal<Vec<JobStatus>>,
    pub(crate) log: Signal<Vec<LogLine>>,
    pub(crate) clock: Signal<&'static str>,
    pub(crate) durations: Signal<Vec<f32>>,
    pub(crate) feed: Signal<Status<(), String>>,
    pub(crate) section: Signal<Section>,
    pub(crate) service: Signal<Service>,
    pub(crate) env_filter: Signal<usize>,
    pub(crate) focus: Signal<usize>,
    pub(crate) paused: Signal<bool>,
    pub(crate) menu_open: Signal<bool>,
    pub(crate) toasts: ToastQueue,
}

#[derive(Clone, Copy)]
pub(crate) struct Rollout {
    pub(crate) region: Signal<usize>,
    pub(crate) canary: Signal<f32>,
    pub(crate) surge: Signal<i32>,
    pub(crate) health_path: Signal<String>,
    pub(crate) auto_rollback: Signal<bool>,
    pub(crate) notify: Signal<bool>,
}

pub(crate) fn rollout() -> Rollout {
    Rollout {
        region: signal(0),
        canary: signal(CANARY_START),
        surge: signal(SURGE_START),
        health_path: signal(HEALTH_PATH.to_owned()),
        auto_rollback: signal(true),
        notify: signal(true),
    }
}

struct Feed {
    statuses: SignalWriter<Vec<JobStatus>>,
    log: SignalWriter<Vec<LogLine>>,
    clock: SignalWriter<&'static str>,
    durations: SignalWriter<Vec<f32>>,
}

impl Feed {
    async fn run(self) -> Result<(), ScheduleError> {
        sleep(Duration::from_millis(START_DELAY_MS)).await;
        for step in SCRIPT {
            if step.after_ms > 0 {
                sleep(Duration::from_millis(step.after_ms)).await;
            }
            self.apply(step.event)?;
        }
        Ok(())
    }

    fn apply(&self, event: Event) -> Result<(), ScheduleError> {
        match event {
            Event::Status(slot, status) => self.statuses.update(move |all| {
                if let Some(entry) = all.get_mut(slot) {
                    *entry = status;
                }
            }),
            Event::Log(line) => self.log.update(move |all| all.push(line)),
            Event::Clock(time) => self.clock.set(time),
            Event::Duration(minutes) => self.durations.update(move |all| {
                all.push(minutes);
                if all.len() > DURATIONS.len() {
                    all.remove(0);
                }
            }),
        }
    }
}

pub(crate) fn start() -> Console {
    let statuses = signal(JOBS.iter().map(|job| job.status).collect::<Vec<_>>());
    let log = signal(LOG_SEED.to_vec());
    let clock = signal(CLOCK_START);
    let durations = signal(DURATIONS.to_vec());
    let backend = Feed {
        statuses: signal_writer(statuses),
        log: signal_writer(log),
        clock: signal_writer(clock),
        durations: signal_writer(durations),
    };
    let feed = spawn(async move {
        match backend.run().await {
            Ok(()) | Err(ScheduleError::Closed) => Ok(()),
            Err(err) => Err(err.to_string()),
        }
    });
    Console {
        statuses,
        log,
        clock,
        durations,
        feed,
        section: signal(Section::Deploys),
        service: signal(Service::Ledger),
        env_filter: signal(0),
        focus: signal(0),
        paused: signal(false),
        menu_open: signal(false),
        toasts: ToastQueue::new(),
    }
}

impl Console {
    pub(crate) fn status_of(self, slot: usize) -> JobStatus {
        let seeded = JOBS.get(slot).map_or(JobStatus::Queued, |job| job.status);
        self.statuses
            .with(|all| all.get(slot).copied().unwrap_or(seeded))
    }

    pub(crate) fn tally(self, service: Option<Service>, bucket: Bucket) -> usize {
        self.statuses.with(|all| {
            JOBS.iter()
                .zip(all)
                .filter(|(job, status)| {
                    service.is_none_or(|wanted| job.service == wanted) && status.bucket() == bucket
                })
                .count()
        })
    }

    pub(crate) fn focused(self) -> Option<&'static Job> {
        JOBS.get(self.focus.get())
    }

    pub(crate) fn link_text(self) -> String {
        match self.feed.get() {
            Status::Err(err) | Status::Panicked(err) => format!("● feed error: {err}"),
            Status::Idle | Status::Pending | Status::Ok(()) => "● connected · us-east".to_owned(),
        }
    }

    fn set_status(self, slot: usize, status: JobStatus) {
        self.statuses.update(|all| {
            if let Some(entry) = all.get_mut(slot) {
                *entry = status;
            }
        });
    }

    pub(crate) fn deploy(self) {
        let service = self.service.peek();
        self.toasts
            .success(format!("Deploy queued for {}", service.name()));
    }

    pub(crate) fn set_paused(self, paused: bool) {
        self.paused.set(paused);
        if paused {
            self.toasts.warning("All deploys paused");
        } else {
            self.toasts.info("Deploys resumed");
        }
    }

    pub(crate) fn retry(self, slot: usize) {
        self.set_status(slot, JobStatus::Queued);
        if let Some(job) = JOBS.get(slot) {
            self.toasts.info(format!("Retry queued for {}", job.sha));
        }
    }

    pub(crate) fn roll_back(self, slot: usize) {
        self.set_status(slot, JobStatus::RolledBack);
        if let Some(job) = JOBS.get(slot) {
            self.toasts
                .warning(format!("Rolled back {} on {}", job.sha, job.region.label()));
        }
    }

    pub(crate) fn go(self, section: Section) {
        self.section.set(section);
        self.menu_open.set(false);
    }
}
