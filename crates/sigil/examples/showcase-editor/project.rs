use sigil::prelude::*;

pub(crate) const PROJECT_NAME: &str = "orbit";
pub(crate) const BRANCH: &str = "feat/rk4";
pub(crate) const AHEAD: u32 = 2;
pub(crate) const BEHIND: u32 = 0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Folder {
    Src,
    Physics,
    Render,
    Tests,
}

impl Folder {
    pub(crate) const ALL: [Self; 4] = [Self::Src, Self::Physics, Self::Render, Self::Tests];

    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Src => "src",
            Self::Physics => "physics",
            Self::Render => "render",
            Self::Tests => "tests",
        }
    }

    pub(crate) const fn parent(self) -> Option<Self> {
        match self {
            Self::Physics | Self::Render => Some(Self::Src),
            Self::Src | Self::Tests => None,
        }
    }

    pub(crate) fn chain(folder: Option<Self>) -> Vec<Self> {
        let mut out = Vec::new();
        let mut cursor = folder;
        while let Some(current) = cursor {
            out.insert(0, current);
            cursor = current.parent();
        }
        out
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum FileId {
    CargoToml,
    Readme,
    Main,
    Lib,
    PhysicsMod,
    Integrator,
    Camera,
    OrbitTest,
}

pub(crate) const FILE_COUNT: usize = 8;

impl FileId {
    pub(crate) const ALL: [Self; FILE_COUNT] = [
        Self::CargoToml,
        Self::Readme,
        Self::Main,
        Self::Lib,
        Self::PhysicsMod,
        Self::Integrator,
        Self::Camera,
        Self::OrbitTest,
    ];

    pub(crate) const fn spec(self) -> FileSpec {
        match self {
            Self::CargoToml => FileSpec::new("Cargo.toml", None, FileKind::Toml)
                .source(include_str!("sample/Cargo.toml.txt")),
            Self::Readme => FileSpec::new("README.md", None, FileKind::Markdown)
                .source(include_str!("sample/README.md.txt"))
                .git(GitStatus::Modified)
                .dirty(),
            Self::Main => FileSpec::new("main.rs", Some(Folder::Src), FileKind::Rust)
                .source(include_str!("sample/main.rs.txt")),
            Self::Lib => FileSpec::new("lib.rs", Some(Folder::Src), FileKind::Rust)
                .source(include_str!("sample/lib.rs.txt")),
            Self::PhysicsMod => FileSpec::new("mod.rs", Some(Folder::Physics), FileKind::Rust)
                .source(include_str!("sample/physics_mod.rs.txt"))
                .git(GitStatus::Modified),
            Self::Integrator => {
                FileSpec::new("integrator.rs", Some(Folder::Physics), FileKind::Rust)
                    .source(include_str!("sample/integrator.rs.txt"))
                    .git(GitStatus::Modified)
                    .dirty()
            }
            Self::Camera => FileSpec::new("camera.rs", Some(Folder::Render), FileKind::Rust)
                .source(include_str!("sample/camera.rs.txt")),
            Self::OrbitTest => FileSpec::new("orbit.rs", Some(Folder::Tests), FileKind::Rust)
                .source(include_str!("sample/orbit_test.rs.txt"))
                .git(GitStatus::Added),
        }
    }

    pub(crate) const fn index(self) -> usize {
        self as usize
    }

    pub(crate) fn path(self) -> String {
        let spec = self.spec();
        let mut parts: Vec<&str> = Folder::chain(spec.folder)
            .into_iter()
            .map(Folder::name)
            .collect();
        parts.push(spec.name);
        parts.join("/")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FileKind {
    Rust,
    Toml,
    Markdown,
}

impl FileKind {
    pub(crate) const fn language(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Toml => "toml",
            Self::Markdown => "markdown",
        }
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Rust => "Rust",
            Self::Toml => "TOML",
            Self::Markdown => "Markdown",
        }
    }

    pub(crate) const fn icon(self) -> Icon {
        match self {
            Self::Rust => Icon::FileCode,
            Self::Toml => Icon::Braces,
            Self::Markdown => Icon::FileText,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GitStatus {
    Modified,
    Added,
}

impl GitStatus {
    pub(crate) const fn letter(self) -> &'static str {
        match self {
            Self::Modified => "M",
            Self::Added => "A",
        }
    }

    pub(crate) const fn tone(self) -> Tone {
        match self {
            Self::Modified => Tone::Warn,
            Self::Added => Tone::Success,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct FileSpec {
    pub(crate) name: &'static str,
    pub(crate) folder: Option<Folder>,
    pub(crate) kind: FileKind,
    pub(crate) git: Option<GitStatus>,
    pub(crate) dirty: bool,
    pub(crate) source: &'static str,
}

impl FileSpec {
    const fn new(name: &'static str, folder: Option<Folder>, kind: FileKind) -> Self {
        Self {
            name,
            folder,
            kind,
            git: None,
            dirty: false,
            source: "",
        }
    }

    const fn source(mut self, source: &'static str) -> Self {
        self.source = source;
        self
    }

    const fn git(mut self, status: GitStatus) -> Self {
        self.git = Some(status);
        self
    }

    const fn dirty(mut self) -> Self {
        self.dirty = true;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum TreeItem {
    Folder(Folder),
    File(FileId),
}
