use std::future::Future;
use std::rc::Rc;

use crate::reactive::{Signal, signal, untrack};
use crate::tasks::current::host_parts;
use crate::tasks::dispatch::spawn_into;
use crate::tasks::handle::OwnerHandle;
use crate::tasks::status::Status;
use crate::tasks::task::TaskHandle;

type Start<T, E, A> = Rc<dyn Fn(A, Signal<Status<T, E>>, Signal<Run>, u64) -> Option<TaskHandle>>;

#[derive(Clone, Default)]
struct Run {
    id: u64,
    handle: Option<TaskHandle>,
}

pub struct Task<T: 'static, E: 'static, A: 'static = ()> {
    status: Signal<Status<T, E>>,
    start: Signal<Start<T, E, A>>,
    current: Signal<Run>,
}

impl<T: 'static, E: 'static, A: 'static> Clone for Task<T, E, A> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static, E: 'static, A: 'static> Copy for Task<T, E, A> {}

#[must_use]
#[track_caller]
pub fn task<T, E, F, Fut>(make: F) -> Task<T, E>
where
    T: Send + 'static,
    E: Send + 'static,
    F: Fn() -> Fut + 'static,
    Fut: Future<Output = Result<T, E>> + Send + 'static,
{
    task_with(move |()| make())
}

#[must_use]
#[track_caller]
pub fn task_with<A, T, E, F, Fut>(make: F) -> Task<T, E, A>
where
    A: 'static,
    T: Send + 'static,
    E: Send + 'static,
    F: Fn(A) -> Fut + 'static,
    Fut: Future<Output = Result<T, E>> + Send + 'static,
{
    let owner = OwnerHandle::current();
    let start: Start<T, E, A> = Rc::new(move |arg, status, current, id| {
        let owner = owner?;
        let parts = host_parts("task")?;
        let (status, current) = (status.into_sendable(), current.into_sendable());
        spawn_into(&parts, owner, make(arg), move |done| {
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
        current: signal(Run::default()),
    }
}

impl<T: 'static, E: 'static> Task<T, E> {
    pub fn run(self) {
        self.run_with(());
    }
}

impl<T: 'static, E: 'static, A: 'static> Task<T, E, A> {
    pub fn run_with(self, arg: A) {
        let id = self.retire();
        self.status.update(|status| *status = Status::Pending);
        let handle = (self.start.peek())(arg, self.status, self.current, id);
        self.current.update(|run| *run = Run { id, handle });
    }

    pub fn cancel(self) {
        let id = self.retire();
        self.current.update(|run| *run = Run { id, handle: None });
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

    #[must_use]
    pub fn status(self) -> Status<T, E>
    where
        T: Clone,
        E: Clone,
    {
        self.status.get()
    }
}
