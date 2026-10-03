use serde::Deserialize;

use super::spec::State;

const CATALOG: &str = include_str!("catalog.toml");

#[derive(Deserialize)]
pub(crate) struct Catalog {
    pub(crate) pad: f32,
    pub(crate) crop_pad: u32,
    pub(crate) cell: u32,
    pub(crate) drag: [f32; 2],
    pub(crate) states: Vec<StateInfo>,
    pub(crate) groups: Vec<Group>,
    pub(crate) components: Vec<Component>,
    pub(crate) examples: Vec<Example>,
}

#[derive(Deserialize)]
pub(crate) struct StateInfo {
    pub(crate) drive: State,
    pub(crate) id: String,
    pub(crate) label: String,
}

#[derive(Deserialize)]
pub(crate) struct Group {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) source: String,
}

#[derive(Deserialize)]
pub(crate) struct Component {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) group: String,
    pub(crate) builder: String,
    #[serde(default)]
    pub(crate) preview: Option<String>,
    #[serde(default)]
    pub(crate) live: Live,
    #[serde(default)]
    pub(crate) captures: Vec<Capture>,
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Live {
    #[default]
    Sandbox,
    Off,
}

#[derive(Deserialize)]
pub(crate) struct Capture {
    pub(crate) build: String,
    #[serde(default)]
    pub(crate) variant: String,
    #[serde(default)]
    pub(crate) label: String,
    pub(crate) size: [u32; 2],
    pub(crate) hit: [f32; 2],
    #[serde(default)]
    pub(crate) grid: Option<[u16; 2]>,
    pub(crate) states: Vec<State>,
}

#[derive(Deserialize)]
pub(crate) struct Example {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) summary: String,
    pub(crate) hosts: Vec<HostKind>,
    pub(crate) build: String,
    pub(crate) size: [u32; 2],
    pub(crate) grid: [u16; 2],
    #[serde(default)]
    pub(crate) settle: Settle,
    pub(crate) source: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum HostKind {
    Window,
    Terminal,
}

#[derive(Clone, Copy, Default, Deserialize)]
pub(crate) struct Settle {
    pub(crate) frames: u32,
    pub(crate) ms: u64,
}

pub(crate) fn load() -> Result<Catalog, String> {
    toml::from_str(CATALOG).map_err(|err| format!("catalog.toml: {err}"))
}

impl Catalog {
    pub(crate) fn state(&self, drive: State) -> Option<&StateInfo> {
        self.states.iter().find(|info| info.drive == drive)
    }
}

impl Capture {
    pub(crate) fn state_id(&self, info: &StateInfo) -> String {
        if self.variant.is_empty() {
            info.id.clone()
        } else {
            format!("{}-{}", self.variant, info.id)
        }
    }

    pub(crate) fn state_label(&self, info: &StateInfo) -> String {
        if self.label.is_empty() {
            info.label.clone()
        } else {
            format!("{}: {}", self.label, info.label)
        }
    }

    pub(crate) fn cells(&self, cell: u32) -> (u16, u16) {
        if let Some(grid) = self.grid {
            return grid.into();
        }
        let cols = u16::try_from(self.size[0] / cell).unwrap_or(u16::MAX);
        let rows = u16::try_from(self.size[1] / cell).unwrap_or(u16::MAX);
        (cols, rows)
    }
}
