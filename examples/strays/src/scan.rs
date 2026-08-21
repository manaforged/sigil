//! The socket table joined to the process table.
//!
//! A row is one process that holds a listening TCP port. Ownership comes
//! from its working directory; whether anyone is watching comes from its
//! controlling terminal.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use netstat2::{AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, TcpState, get_sockets_info};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

/// Directory roots that count as source trees.
pub(crate) const DEV: &str = "/path/to/dev";

/// How long a dev-tree server may run unattended before it is a stray.
#[allow(
    clippy::duration_suboptimal_units,
    reason = "Duration::from_hours requires Rust 1.91; Sigil supports 1.90"
)]
pub(crate) const LIMIT: Duration = Duration::from_secs(4 * 3600);

/// What the row is, once age, terminal, and location are read together.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum Verdict {
    /// Outside every dev root. Not this tool's business.
    System,
    /// A terminal is attached, so somebody is watching it.
    Held,
    /// In a dev tree, unattended, still inside the limit.
    Fresh,
    /// In a dev tree, unattended, past the limit.
    Stray,
}

impl Verdict {
    /// Header text for the state column.
    pub(crate) const fn title(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Held => "held",
            Self::Fresh => "fresh",
            Self::Stray => "STRAY",
        }
    }
}

/// One listening process, resolved back to the work it belongs to.
#[derive(Clone, Debug)]
pub(crate) struct Stray {
    /// Process id. Stable for this row's lifetime, so it is the row id.
    pub(crate) pid: u32,
    /// Process group. Killing the group kills the `npm` that owns the leaf.
    pub(crate) pgid: u32,
    /// Listening TCP ports, ascending.
    pub(crate) ports: Vec<u16>,
    /// Time since the process started.
    pub(crate) age: Duration,
    /// Resident set size in bytes.
    pub(crate) rss: u64,
    /// Working directory, when the kernel will report it.
    pub(crate) cwd: Option<PathBuf>,
    /// Argv joined by spaces.
    pub(crate) cmd: String,
    /// A controlling terminal is attached.
    pub(crate) tty: bool,
}

impl Stray {
    /// Classify against a dev root and an age limit.
    pub(crate) fn verdict(&self, root: &Path, limit: Duration) -> Verdict {
        let inside = self.cwd.as_deref().is_some_and(|c| c.starts_with(root));
        if !inside {
            Verdict::System
        } else if self.tty {
            Verdict::Held
        } else if self.age > limit {
            Verdict::Stray
        } else {
            Verdict::Fresh
        }
    }

    /// Directory name of the working directory, or `?`.
    pub(crate) fn place(&self) -> String {
        self.cwd
            .as_deref()
            .and_then(Path::file_name)
            .map_or_else(|| String::from("?"), |n| n.to_string_lossy().into_owned())
    }

    /// Ports as `5174,5175`.
    pub(crate) fn port_list(&self) -> String {
        self.ports
            .iter()
            .map(u16::to_string)
            .collect::<Vec<_>>()
            .join(",")
    }
}

/// Reads the machine, keeping what does not change between reads.
///
/// A scan touches only the processes that hold a listening port, and asks the
/// system for a process group and terminal only for a pid it has not seen.
/// Neither of those changes over a process's life.
pub(crate) struct Scanner {
    system: System,
    known: HashMap<u32, (u32, bool)>,
}

impl Scanner {
    /// An empty scanner. The first [`Self::read`] fills it.
    pub(crate) fn new() -> Self {
        Self {
            system: System::new(),
            known: HashMap::new(),
        }
    }

    /// Every listening process this user can see, oldest first.
    pub(crate) fn read(&mut self) -> Vec<Stray> {
        let ports = listening_ports();

        // Read the fields this list shows, for these processes only. Refreshing
        // every process on the machine to look up a dozen costs milliseconds.
        let pids: Vec<Pid> = ports.keys().map(|pid| Pid::from_u32(*pid)).collect();
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&pids),
            false,
            ProcessRefreshKind::nothing()
                .with_memory()
                .with_cwd(UpdateKind::OnlyIfNotSet)
                .with_cmd(UpdateKind::OnlyIfNotSet),
        );
        self.learn(&ports);

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let mut rows: Vec<Stray> = ports
            .into_iter()
            .filter_map(|(pid, listed)| self.row(pid, listed, now))
            .collect();
        rows.sort_by_key(|row| core::cmp::Reverse(row.age));
        rows
    }

    /// One row, or `None` when the process died between the port scan
    /// and the refresh.
    fn row(&self, pid: u32, mut listed: Vec<u16>, now: u64) -> Option<Stray> {
        let process = self.system.process(Pid::from_u32(pid))?;
        let (group, tty) = self.known.get(&pid).copied().unwrap_or((pid, false));
        listed.sort_unstable();
        listed.dedup();
        Some(Stray {
            pid,
            pgid: group,
            ports: listed,
            age: Duration::from_secs(now.saturating_sub(process.start_time())),
            rss: process.memory(),
            cwd: process.cwd().map(PathBuf::from),
            cmd: process
                .cmd()
                .iter()
                .map(|part| part.to_string_lossy())
                .collect::<Vec<_>>()
                .join(" "),
            tty,
        })
    }

    /// Learn the process group and terminal of any pid not seen before.
    ///
    /// One `ps` call covers every new pid. A steady list makes none: the fork
    /// costs more than the rest of a scan put together.
    fn learn(&mut self, listeners: &HashMap<u32, Vec<u16>>) {
        self.known.retain(|pid, _| listeners.contains_key(pid));
        let fresh: Vec<String> = listeners
            .keys()
            .filter(|pid| !self.known.contains_key(pid))
            .map(u32::to_string)
            .collect();
        if fresh.is_empty() {
            return;
        }
        let Ok(out) = std::process::Command::new("ps")
            .args(["-o", "pid=,pgid=,tty=", "-p", &fresh.join(",")])
            .output()
        else {
            return;
        };
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            let mut fields = line.split_whitespace();
            let Some(pid) = fields.next().and_then(|v| v.parse().ok()) else {
                continue;
            };
            let Some(group) = fields.next().and_then(|v| v.parse().ok()) else {
                continue;
            };
            let tty = fields
                .next()
                .is_some_and(|name| name != "??" && name != "?");
            self.known.insert(pid, (group, tty));
        }
    }
}

fn listening_ports() -> HashMap<u32, Vec<u16>> {
    let mut ports: HashMap<u32, Vec<u16>> = HashMap::new();
    let sockets =
        get_sockets_info(AddressFamilyFlags::all(), ProtocolFlags::TCP).unwrap_or_default();
    for socket in sockets {
        let ProtocolSocketInfo::Tcp(tcp) = &socket.protocol_socket_info else {
            continue;
        };
        if tcp.state != TcpState::Listen {
            continue;
        }
        for pid in &socket.associated_pids {
            ports.entry(*pid).or_default().push(tcp.local_port);
        }
    }
    ports.remove(&std::process::id());
    ports
}

/// Terminate the process group, then insist.
///
/// The leaf `vite` is not the owner. Its `npm run dev` parent holds the
/// port open when only the leaf dies. A negative argument to `kill`
/// addresses the whole group.
pub(crate) fn kill(pgid: u32) {
    if pgid == 0 {
        return;
    }
    drop(
        std::process::Command::new("kill")
            .args(["-TERM", &format!("-{pgid}")])
            .status(),
    );
}
