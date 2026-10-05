use sigil::prelude::*;
use sigil::widgets::{ChangeKind, DiagSeverity};

use super::project::FileId;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Diagnostic {
    pub(crate) file: FileId,
    pub(crate) line: usize,
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) severity: DiagSeverity,
    pub(crate) code: &'static str,
    pub(crate) message: &'static str,
}

pub(crate) const DIAGNOSTICS: &[Diagnostic] = &[
    Diagnostic {
        file: FileId::Integrator,
        line: 36,
        start: 12,
        end: 19,
        severity: DiagSeverity::Warning,
        code: "unused_variables",
        message: "unused variable: `half_dt`",
    },
    Diagnostic {
        file: FileId::Main,
        line: 15,
        start: 10,
        end: 30,
        severity: DiagSeverity::Warning,
        code: "cast_precision_loss",
        message: "casting `u64` to `f64` may lose precision",
    },
    Diagnostic {
        file: FileId::Integrator,
        line: 102,
        start: 53,
        end: 57,
        severity: DiagSeverity::Info,
        code: "float_div",
        message: "`dist` is zero when two bodies share a position",
    },
];

pub(crate) const fn severity_icon(severity: DiagSeverity) -> Icon {
    match severity {
        DiagSeverity::Error => Icon::CircleX,
        DiagSeverity::Warning => Icon::TriangleAlert,
        DiagSeverity::Info | DiagSeverity::Hint => Icon::Info,
    }
}

pub(crate) const fn severity_tone(severity: DiagSeverity) -> Tone {
    match severity {
        DiagSeverity::Error => Tone::Error,
        DiagSeverity::Warning => Tone::Warn,
        DiagSeverity::Info | DiagSeverity::Hint => Tone::Info,
    }
}

pub(crate) fn count(severity: DiagSeverity) -> usize {
    DIAGNOSTICS
        .iter()
        .filter(|diag| diag.severity == severity)
        .count()
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Change {
    pub(crate) file: FileId,
    pub(crate) first: usize,
    pub(crate) last: usize,
    pub(crate) kind: ChangeKind,
}

pub(crate) const CHANGES: &[Change] = &[
    Change {
        file: FileId::Integrator,
        first: 6,
        last: 9,
        kind: ChangeKind::Added,
    },
    Change {
        file: FileId::Integrator,
        first: 27,
        last: 30,
        kind: ChangeKind::Modified,
    },
    Change {
        file: FileId::Integrator,
        first: 47,
        last: 61,
        kind: ChangeKind::Added,
    },
    Change {
        file: FileId::Integrator,
        first: 85,
        last: 85,
        kind: ChangeKind::Deleted,
    },
    Change {
        file: FileId::PhysicsMod,
        first: 3,
        last: 3,
        kind: ChangeKind::Modified,
    },
    Change {
        file: FileId::Readme,
        first: 4,
        last: 8,
        kind: ChangeKind::Added,
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SymbolKind {
    Const,
    Enum,
    Struct,
    Impl,
    Function,
}

impl SymbolKind {
    pub(crate) const fn icon(self) -> Icon {
        match self {
            Self::Const => Icon::Hash,
            Self::Enum => Icon::List,
            Self::Struct => Icon::Box,
            Self::Impl => Icon::Layers,
            Self::Function => Icon::Braces,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Symbol {
    pub(crate) file: FileId,
    pub(crate) kind: SymbolKind,
    pub(crate) name: &'static str,
    pub(crate) line: usize,
    pub(crate) depth: u16,
}

const fn sym(
    file: FileId,
    kind: SymbolKind,
    name: &'static str,
    line: usize,
    depth: u16,
) -> Symbol {
    Symbol {
        file,
        kind,
        name,
        line,
        depth,
    }
}

pub(crate) const SYMBOLS: &[Symbol] = &[
    sym(FileId::Integrator, SymbolKind::Const, "GRAVITY", 2, 0),
    sym(FileId::Integrator, SymbolKind::Const, "SOFTENING", 3, 0),
    sym(FileId::Integrator, SymbolKind::Enum, "Scheme", 6, 0),
    sym(FileId::Integrator, SymbolKind::Struct, "Integrator", 11, 0),
    sym(
        FileId::Integrator,
        SymbolKind::Impl,
        "impl Integrator",
        17,
        0,
    ),
    sym(FileId::Integrator, SymbolKind::Function, "new", 18, 1),
    sym(FileId::Integrator, SymbolKind::Function, "steps", 22, 1),
    sym(FileId::Integrator, SymbolKind::Function, "step", 26, 1),
    sym(FileId::Integrator, SymbolKind::Function, "verlet", 34, 1),
    sym(FileId::Integrator, SymbolKind::Function, "rk4", 47, 1),
    sym(
        FileId::Integrator,
        SymbolKind::Function,
        "accelerations",
        63,
        0,
    ),
    sym(
        FileId::Integrator,
        SymbolKind::Function,
        "derivative",
        79,
        0,
    ),
    sym(FileId::Integrator, SymbolKind::Function, "advance", 84, 0),
    sym(
        FileId::Integrator,
        SymbolKind::Function,
        "total_energy",
        96,
        0,
    ),
    sym(FileId::Main, SymbolKind::Const, "SUN_MASS", 4, 0),
    sym(FileId::Main, SymbolKind::Const, "EARTH_MASS", 5, 0),
    sym(FileId::Main, SymbolKind::Const, "AU", 6, 0),
    sym(FileId::Main, SymbolKind::Const, "DAY", 7, 0),
    sym(FileId::Main, SymbolKind::Function, "main", 9, 0),
    sym(FileId::Lib, SymbolKind::Struct, "World", 5, 0),
    sym(FileId::Lib, SymbolKind::Impl, "impl World", 10, 0),
    sym(FileId::Lib, SymbolKind::Function, "new", 11, 1),
    sym(FileId::Lib, SymbolKind::Function, "spawn", 15, 1),
    sym(FileId::Lib, SymbolKind::Function, "tick", 19, 1),
    sym(FileId::Lib, SymbolKind::Function, "steps", 23, 1),
    sym(FileId::PhysicsMod, SymbolKind::Struct, "Body", 7, 0),
    sym(FileId::PhysicsMod, SymbolKind::Impl, "impl Body", 13, 0),
    sym(FileId::PhysicsMod, SymbolKind::Function, "new", 14, 1),
    sym(FileId::PhysicsMod, SymbolKind::Function, "momentum", 18, 1),
    sym(FileId::Camera, SymbolKind::Struct, "Camera", 2, 0),
    sym(FileId::Camera, SymbolKind::Impl, "impl Camera", 8, 0),
    sym(FileId::Camera, SymbolKind::Function, "looking_at", 9, 1),
    sym(FileId::Camera, SymbolKind::Function, "project", 13, 1),
    sym(FileId::OrbitTest, SymbolKind::Function, "two_body", 3, 0),
    sym(
        FileId::OrbitTest,
        SymbolKind::Function,
        "verlet_conserves_energy",
        11,
        0,
    ),
    sym(
        FileId::OrbitTest,
        SymbolKind::Function,
        "rk4_tracks_verlet",
        21,
        0,
    ),
];

pub(crate) const PROMPT: &str = "orbit (feat/rk4) $ ";

pub(crate) const TERMINAL: &[&str] = &[
    "orbit (feat/rk4) $ cargo test",
    "   Compiling orbit v0.3.0",
    "warning: unused variable: `half_dt`",
    "  --> src/physics/integrator.rs:37:13",
    "    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.41s",
    "     Running unittests src/lib.rs",
    "",
    "running 0 tests",
    "",
    "     Running tests/orbit.rs",
    "",
    "running 2 tests",
    "test rk4_tracks_verlet ... ok (0.84s)",
    "test verlet_conserves_energy ... ok (1.12s)",
    "",
    "test result: ok. 2 passed; 0 failed; 0 ignored; finished in 1.13s",
    "",
    PROMPT,
];

pub(crate) const OUTPUT: &[&str] = &[
    "[check] indexing orbit: 8 files",
    "[check] cargo check finished in 0.92s",
    "[check] 2 warnings, 1 note",
    "[git] fetched origin: up to date",
    "[git] feat/rk4 is 2 commits ahead of main",
    "[fmt] src/physics/integrator.rs formatted on save",
];
