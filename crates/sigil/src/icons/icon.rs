macro_rules! lucide_icons {
    ($($variant:ident => $name:literal,)*) => {
        #[non_exhaustive]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum Icon {
            $($variant,)*
        }

        impl Icon {
            pub const ALL: &'static [Self] = &[$(Self::$variant,)*];

            #[must_use]
            pub const fn name(self) -> &'static str {
                match self {
                    $(Self::$variant => $name,)*
                }
            }

            #[must_use]
            pub const fn svg(self) -> &'static str {
                match self {
                    $(Self::$variant => include_str!(concat!("../../assets/icons/lucide/", $name, ".svg")),)*
                }
            }
        }
    };
}

lucide_icons! {
    Activity => "activity",
    ArrowDown => "arrow-down",
    ArrowLeft => "arrow-left",
    ArrowRight => "arrow-right",
    ArrowUp => "arrow-up",
    AtSign => "at-sign",
    Bell => "bell",
    Box => "box",
    Braces => "braces",
    Bug => "bug",
    Calendar => "calendar",
    ChartColumn => "chart-column",
    Check => "check",
    ChevronDown => "chevron-down",
    ChevronLeft => "chevron-left",
    ChevronRight => "chevron-right",
    ChevronUp => "chevron-up",
    ChevronsUpDown => "chevrons-up-down",
    CircleAlert => "circle-alert",
    CircleCheck => "circle-check",
    CircleX => "circle-x",
    Circle => "circle",
    Clock => "clock",
    Cloud => "cloud",
    Code => "code",
    Command => "command",
    Copy => "copy",
    Cpu => "cpu",
    Database => "database",
    Download => "download",
    EllipsisVertical => "ellipsis-vertical",
    Ellipsis => "ellipsis",
    ExternalLink => "external-link",
    EyeOff => "eye-off",
    Eye => "eye",
    FileCode => "file-code",
    FileText => "file-text",
    File => "file",
    FolderOpen => "folder-open",
    Folder => "folder",
    Funnel => "funnel",
    Gauge => "gauge",
    GitBranch => "git-branch",
    GitCommitHorizontal => "git-commit-horizontal",
    GitPullRequest => "git-pull-request",
    Globe => "globe",
    GripVertical => "grip-vertical",
    Hash => "hash",
    Heart => "heart",
    History => "history",
    House => "house",
    Inbox => "inbox",
    Info => "info",
    Key => "key",
    Layers => "layers",
    LayoutDashboard => "layout-dashboard",
    Link => "link",
    ListTree => "list-tree",
    List => "list",
    LockOpen => "lock-open",
    Lock => "lock",
    LogIn => "log-in",
    LogOut => "log-out",
    Mail => "mail",
    Menu => "menu",
    Minus => "minus",
    Monitor => "monitor",
    Moon => "moon",
    Package => "package",
    PanelBottom => "panel-bottom",
    PanelLeft => "panel-left",
    Pause => "pause",
    Pencil => "pencil",
    Play => "play",
    Plus => "plus",
    RefreshCw => "refresh-cw",
    Rocket => "rocket",
    RotateCcw => "rotate-ccw",
    ScrollText => "scroll-text",
    Search => "search",
    Server => "server",
    Settings => "settings",
    Shield => "shield",
    SlidersHorizontal => "sliders-horizontal",
    Square => "square",
    Star => "star",
    Sun => "sun",
    Terminal => "terminal",
    Trash2 => "trash-2",
    TrendingDown => "trending-down",
    TrendingUp => "trending-up",
    TriangleAlert => "triangle-alert",
    Triangle => "triangle",
    Upload => "upload",
    User => "user",
    Users => "users",
    X => "x",
    Zap => "zap",
}
