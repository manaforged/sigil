use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, LazyLock, Mutex, PoisonError};

use tokio::sync::oneshot;

use crate::fnv::{OFFSET, mix};

use super::script::Script;

const STACK: usize = 32 << 20;
const MAX_CHARTS: usize = 48;
const MAX_BYTES: usize = 48 << 20;

pub(super) struct Chart {
    pub(super) svg: String,
    pub(super) text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Failure {
    Deadline,
    Memory,
    Script,
    Engine,
}

impl Failure {
    const fn hits_a_cap(self) -> bool {
        matches!(self, Self::Deadline | Self::Memory)
    }
}

pub(super) type Outcome = Result<Arc<Chart>, Failure>;

type Key = (u64, u64);

#[derive(Clone)]
pub(super) struct Job {
    spec: Arc<str>,
    config: Arc<str>,
    key: Key,
}

impl Job {
    pub(super) fn new(spec: &Arc<str>, source: u64, config: String) -> Self {
        let theme = config.bytes().fold(OFFSET, |key, byte| mix(key, u64::from(byte)));
        Self {
            spec: Arc::clone(spec),
            config: config.into(),
            key: (source, theme),
        }
    }

    pub(super) const fn theme(&self) -> u64 {
        self.key.1
    }
}

enum Reply {
    Task(oneshot::Sender<Outcome>),
    Wait(Sender<Outcome>),
}

impl Reply {
    fn wanted(&self) -> bool {
        match self {
            Self::Task(sender) => !sender.is_closed(),
            Self::Wait(_) => true,
        }
    }

    fn send(self, outcome: Outcome) {
        match self {
            Self::Task(sender) => drop(sender.send(outcome)),
            Self::Wait(sender) => drop(sender.send(outcome)),
        }
    }
}

#[derive(Default)]
struct Cache {
    charts: HashMap<Key, Outcome>,
    order: VecDeque<Key>,
    bytes: usize,
}

impl Cache {
    fn put(&mut self, key: Key, outcome: Outcome) {
        self.bytes += size(&outcome);
        match self.charts.insert(key, outcome) {
            Some(old) => self.bytes = self.bytes.saturating_sub(size(&old)),
            None => self.order.push_back(key),
        }
        while self.order.len() > MAX_CHARTS || self.bytes > MAX_BYTES {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            if let Some(gone) = self.charts.remove(&oldest) {
                self.bytes = self.bytes.saturating_sub(size(&gone));
            }
        }
    }
}

static CACHE: LazyLock<Mutex<Cache>> = LazyLock::new(Mutex::default);
static QUEUE: LazyLock<Option<Sender<(Job, Reply)>>> = LazyLock::new(start);

fn size(outcome: &Outcome) -> usize {
    outcome.as_ref().map_or(0, |chart| chart.svg.len() + chart.text.len())
}

pub(super) fn cached(job: &Job) -> Option<Outcome> {
    let cache = CACHE.lock().unwrap_or_else(PoisonError::into_inner);
    cache.charts.get(&job.key).cloned()
}

pub(super) async fn render(jobs: Vec<Job>) -> Outcome {
    let mut first = None;
    for job in jobs {
        let (sender, receiver) = oneshot::channel();
        submit(job, Reply::Task(sender))?;
        let outcome = receiver.await.unwrap_or(Err(Failure::Engine));
        let chart = first.get_or_insert(outcome).clone()?;
        drop(chart);
    }
    first.unwrap_or(Err(Failure::Engine))
}

pub(super) fn render_now(job: Job) -> Outcome {
    if let Some(outcome) = cached(&job) {
        return outcome;
    }
    let (sender, receiver) = mpsc::channel();
    submit(job, Reply::Wait(sender))?;
    receiver.recv().unwrap_or(Err(Failure::Engine))
}

fn submit(job: Job, reply: Reply) -> Result<(), Failure> {
    let queue = QUEUE.as_ref().ok_or(Failure::Engine)?;
    queue.send((job, reply)).map_err(|_| Failure::Engine)
}

fn start() -> Option<Sender<(Job, Reply)>> {
    let (sender, receiver) = mpsc::channel();
    std::thread::Builder::new()
        .name("sigil-vega".to_string())
        .stack_size(STACK)
        .spawn(move || serve(&receiver))
        .ok()?;
    Some(sender)
}

fn serve(receiver: &Receiver<(Job, Reply)>) {
    let mut script = None;
    for (job, reply) in receiver {
        if !reply.wanted() {
            continue;
        }
        let outcome = cached(&job).unwrap_or_else(|| run(&mut script, &job));
        reply.send(outcome);
    }
}

fn run(script: &mut Option<Script>, job: &Job) -> Outcome {
    if script.is_none() {
        *script = Script::new();
    }
    let outcome = script
        .as_ref()
        .ok_or(Failure::Engine)
        .and_then(|engine| engine.render(&job.spec, &job.config))
        .map(Arc::new);
    if outcome.as_ref().err().is_some_and(|failure| failure.hits_a_cap()) {
        *script = None;
    }
    if !matches!(outcome, Err(Failure::Engine)) {
        let mut cache = CACHE.lock().unwrap_or_else(PoisonError::into_inner);
        cache.put(job.key, outcome.clone());
    }
    outcome
}
