use std::future::Future;
use std::rc::Rc;

use crate::reactive::Owner;
use crate::reactive::{Signal, signal, untrack};
use crate::tasks::current::host_parts;
use crate::tasks::dispatch::spawn_into;
use crate::tasks::status::Status;
use crate::tasks::task::TaskHandle;

type Start<T, E> = Rc<dyn Fn(Signal<Status<T, E>>, Signal<TaskRun>, u64) -> Option<TaskHandle>>;

#[derive(Clone, Default)]
struct TaskRun {
    id: u64,
    handle: Option<TaskHandle>,
}

/// Async work that runs again on `run` and cancels its previous run.
pub struct Task<T: 'static, E: 'static> {
    status: Signal<Status<T, E>>,
    start: Signal<Start<T, E>>,
    current: Signal<TaskRun>,
}

impl<T: 'static, E: 'static> Clone for Task<T, E> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static, E: 'static> Copy for Task<T, E> {}

/// Creates a task that can run again and cancel its previous run.
#[must_use]
#[track_caller]
pub fn task<T, E, F, Fut>(make: F) -> Task<T, E>
where
    T: Send + 'static,
    E: Send + 'static,
    F: Fn() -> Fut + 'static,
    Fut: Future<Output = Result<T, E>> + Send + 'static,
{
    let owner = Owner::current();
    let start: Start<T, E> = Rc::new(move |status, current, id| {
        let owner = owner?;
        let parts = host_parts("task")?;
        let (status, current) = (status.into_sendable(), current.into_sendable());
        spawn_into(&parts, owner, make(), move |done| {
            if current
                .into_signal()
                .try_peek()
                .is_some_and(|run| run.id == id)
            {
                status.into_signal().try_set_always(done);
            }
        })
    });
    Task {
        status: signal(Status::Idle),
        start: signal(start),
        current: signal(TaskRun::default()),
    }
}

impl<T: 'static, E: 'static> Task<T, E> {
    /// Starts the task again.
    pub fn run(self) {
        let id = self.retire();
        self.status.update(|status| *status = Status::Pending);
        let handle = (self.start.peek())(self.status, self.current, id);
        self.current.update(|run| *run = TaskRun { id, handle });
    }

    /// Cancels the task.
    pub fn cancel(self) {
        let id = self.retire();
        self.current
            .update(|run| *run = TaskRun { id, handle: None });
        if untrack(|| self.status.with(Status::is_pending)) {
            self.status.update(|status| *status = Status::Idle);
        }
    }

    fn retire(self) -> u64 {
        let previous = self.current.peek();
        if let Some(handle) = previous.handle {
            handle.abort();
        }
        previous.id + 1
    }

    /// Returns the status.
    #[must_use]
    pub fn status(self) -> Status<T, E>
    where
        T: Clone,
        E: Clone,
    {
        self.status.get()
    }
}
