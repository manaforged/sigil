use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum ScheduleError {
    Closed,
    Full,
}

impl fmt::Display for ScheduleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed => f.write_str("main-thread dispatcher is closed"),
            Self::Full => f.write_str("dispatcher queue is full"),
        }
    }
}

impl Error for ScheduleError {}
