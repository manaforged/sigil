use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use netstat2::{AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, TcpState, get_sockets_info};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

pub(crate) fn dev_root() -> PathBuf {
    if let Some(root) = std::env::var_os("STRAYS_DEV_ROOT") {
        return PathBuf::from(root);
    }
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
        .join("dev")
}

#[allow(
    clippy::duration_suboptimal_units,
    reason = "Duration::from_hours requires Rust 1.91; Sigil supports 1.90"
)]
pub(crate) const LIMIT: Duration = Duration::from_secs(4 * 3600);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum Verdict {
    System,
    Held,
    Fresh,
    Stray,
}

impl Verdict {
    pub(crate) const fn title(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Held => "held",
            Self::Fresh => "fresh",
            Self::Stray => "STRAY",
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Stray {
    pub(crate) pid: u32,
    pub(crate) pgid: u32,
    pub(crate) ports: Vec<u16>,
    pub(crate) age: Duration,
    pub(crate) rss: u64,
    pub(crate) cwd: Option<PathBuf>,
    pub(crate) cmd: String,
    pub(crate) tty: bool,
}

impl Stray {
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

    pub(crate) fn place(&self) -> String {
        self.cwd
            .as_deref()
            .and_then(Path::file_name)
            .map_or_else(|| String::from("?"), |n| n.to_string_lossy().into_owned())
    }

    pub(crate) fn port_list(&self) -> String {
        self.ports
            .iter()
            .map(u16::to_string)
            .collect::<Vec<_>>()
            .join(",")
    }
}

pub(crate) struct Scanner {
    system: System,
    known: HashMap<u32, (u32, bool)>,
}

impl Scanner {
    pub(crate) fn new() -> Self {
        Self {
            system: System::new(),
            known: HashMap::new(),
        }
    }

    pub(crate) fn read(&mut self) -> Vec<Stray> {
        let ports = listening_ports();

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
