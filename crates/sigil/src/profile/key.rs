use std::fmt;

macro_rules! named {
    ($ty:ident { $($variant:ident => $name:literal),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        #[non_exhaustive]
        pub enum $ty {
            $($variant),+
        }

        impl $ty {
            #[must_use]
            pub const fn name(self) -> &'static str {
                match self {
                    $(Self::$variant => $name),+
                }
            }
        }

        impl fmt::Display for $ty {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.pad(self.name())
            }
        }
    };
}

named!(Category {
    Layout => "layout",
    Paint => "paint",
    Render => "render",
    A11y => "a11y",
    Reactive => "reactive",
    App => "app",
});

named!(Phase {
    Effect => "effect",
    Layout => "layout",
    LayoutSolve => "layout.solve",
    LayoutAttach => "layout.attach",
    LayoutSync => "layout.sync",
    LayoutSig => "layout.sig",
    LayoutDirty => "layout.dirty",
    Emit => "emit",
    Paint => "paint",
    RenderCpu => "render.cpu",
    RenderCompositePipeline => "render.pipeline.composite",
    A11y => "a11y",
    TuiPaint => "tui.paint",
    TuiRaster => "tui.raster",
    TuiWrite => "tui.write",
    TuiPresent => "tui.present",
    AppUpdate => "app.update",
});

impl Phase {
    #[must_use]
    pub const fn category(self) -> Category {
        match self {
            Self::Layout
            | Self::LayoutSolve
            | Self::LayoutAttach
            | Self::LayoutSync
            | Self::LayoutSig
            | Self::LayoutDirty => Category::Layout,
            Self::Emit | Self::Paint => Category::Paint,
            Self::RenderCpu | Self::RenderCompositePipeline => Category::Render,
            Self::A11y => Category::A11y,
            Self::Effect => Category::Reactive,
            Self::TuiPaint
            | Self::TuiRaster
            | Self::TuiWrite
            | Self::TuiPresent
            | Self::AppUpdate => Category::App,
        }
    }

    #[must_use]
    pub const fn is_top_level(self) -> bool {
        matches!(
            self,
            Self::Effect | Self::Layout | Self::Emit | Self::Paint | Self::A11y
        )
    }
}

named!(Cause {
    Idle => "idle",
    Pointer => "pointer",
    Hover => "hover",
    Key => "key",
    Scroll => "scroll",
    Ime => "ime",
    Resize => "resize",
    A11y => "a11y",
    Expose => "expose",
    Input => "input",
});

impl Cause {
    #[must_use]
    pub const fn rank(self) -> u8 {
        match self {
            Self::Idle => 0,
            Self::Expose | Self::Input => 1,
            Self::Hover | Self::Resize | Self::A11y => 3,
            Self::Scroll | Self::Ime => 4,
            Self::Pointer | Self::Key => 5,
        }
    }
}

named!(Counter {
    SceneEmit => "scene.emit",
    SceneReplay => "scene.replay",
    SceneCmds => "scene.cmds",
    SceneGlyphs => "scene.glyphs",
    SceneRects => "scene.rects",
    SceneImages => "scene.images",
    RenderFrames => "render.frames",
    GpuRects => "gpu.rects",
    GpuGlyphs => "gpu.glyphs",
    GpuSprites => "gpu.sprites",
    GpuLayers => "gpu.layers",
    GpuLayerPixels => "gpu.layer_pixels",
    GpuRedrawnPixels => "gpu.redrawn_pixels",
    GpuUploadBuffers => "gpu.upload_buffers",
    GpuUploadCapacityBytes => "gpu.upload_capacity_bytes",
    GpuUploadWrites => "gpu.upload_writes",
    GpuUploadBytes => "gpu.upload_bytes",
    GpuLayerCacheBytes => "gpu.layer_cache_bytes",
    GpuLayerCacheTextures => "gpu.layer_cache_textures",
    GpuLayerCacheBudgetBytes => "gpu.layer_cache_budget_bytes",
    GpuLayerReuses => "gpu.layer_reuses",
    GpuPassReuses => "gpu.pass_reuses",
    GpuRenderPasses => "gpu.render_passes",
    GpuSplitSubmissions => "gpu.split_submissions",
    GpuSharedUploadWrites => "gpu.shared_upload_writes",
    GpuSharedUploadBytes => "gpu.shared_upload_bytes",
    GpuCompositePipelines => "gpu.pipeline.composite.created",
    AllocLayerTextures => "alloc.layer.textures",
    AllocLayerBytes => "alloc.layer.bytes",
    AllocUploadBuffers => "alloc.upload.buffers",
    AllocUploadBytes => "alloc.upload.bytes",
    AllocCompositeUniformBindGroups => "alloc.composite.uniform_bind_groups",
    AllocCompositeTextureBindGroups => "alloc.composite.texture_bind_groups",
    AllocRasterBindGroups => "alloc.raster.bind_groups",
    AllocSpriteBindGroups => "alloc.sprite.bind_groups",
    WgpuBuffers => "wgpu.ids.buffers",
    WgpuTextures => "wgpu.ids.textures",
    WgpuTextureViews => "wgpu.ids.texture_views",
    WgpuBindGroups => "wgpu.ids.bind_groups",
    WgpuCommandBuffers => "wgpu.ids.command_buffers",
    WgpuRenderPipelines => "wgpu.ids.render_pipelines",
    LayoutHit => "layout.hit",
    LayoutMiss => "layout.miss",
    LayoutFull => "layout.full",
    LayoutFullUnmirrored => "layout.full.unmirrored",
    LayoutFullNoDirty => "layout.full.nodirty",
    LayoutFullMajority => "layout.full.majority",
    LayoutIncr => "layout.incr",
    LayoutResize => "layout.resize",
    LayoutTranslate => "layout.translate",
    LayoutDirtyNodes => "layout.dirty.n",
    LayoutResizedNodes => "layout.resized.n",
    LayoutRoots => "layout.roots",
    LayoutOverlay => "layout.overlay",
    LayoutIntrinsicMiss => "layout.intrinsic.miss",
    TextMeasure => "text.measure",
    PathRaster => "path.raster",
    TuiFrames => "tui.frames",
    TuiEvents => "tui.events",
    TuiDirty => "tui.dirty",
    TuiNodes => "tui.nodes",
    TuiRowsWritten => "tui.rows_written",
    TuiBytes => "tui.bytes",
    TuiWrites => "tui.writes",
    WorkloadCols => "workload.cols",
    WorkloadRows => "workload.rows",
    WorkloadItems => "workload.items",
    WorkloadHeadless => "workload.headless",
    WorkloadFps => "workload.fps",
    WorkloadScenario => "workload.scenario",
    BuildProfile => "build.profile",
});

named!(BackendKind {
    Gpu => "gpu",
    GpuSoftware => "gpu-software",
    Cpu => "cpu",
    Terminal => "terminal",
});

named!(FallbackCause {
    None => "none",
    NoAdapter => "no-adapter",
    DeviceError => "device-error",
    ForcedByEnv => "forced-by-env",
    UnknownEnv => "unknown-env",
});

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BackendStatus {
    pub kind: BackendKind,
    pub fallback: FallbackCause,
}

impl BackendStatus {
    pub const TERMINAL: Self = Self {
        kind: BackendKind::Terminal,
        fallback: FallbackCause::None,
    };
}

impl fmt::Display for BackendStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.fallback {
            FallbackCause::None => write!(f, "{}", self.kind),
            cause => write!(f, "{} ({cause})", self.kind),
        }
    }
}

pub type PhaseTime = (Phase, u64);
pub type CounterValue = (Counter, u64);

pub(crate) fn accumulate<K: Copy + PartialEq>(list: &mut Vec<(K, u64)>, key: K, add: u64) {
    if let Some((_, ns)) = list.iter_mut().find(|(k, _)| *k == key) {
        *ns = ns.saturating_add(add);
    } else {
        list.push((key, add));
    }
}
