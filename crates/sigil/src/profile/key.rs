use std::fmt;

macro_rules! named {
    ($(#[$attr:meta])* kittest $ty:ident $body:tt) => {
        #[cfg(feature = "kittest")]
        named!($(#[$attr])* pub $ty $body);
        #[cfg(not(feature = "kittest"))]
        named!($(#[$attr])* pub(crate) $ty $body);
    };
    ($(#[$attr:meta])* $vis:vis $ty:ident { $($(#[$vattr:meta])* $variant:ident $(if $cond:meta)? => $name:literal),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        $(#[$attr])*
        #[non_exhaustive]
        $vis enum $ty {
            $($(#[$vattr])* $(#[cfg($cond)])? #[doc = concat!("The `", $name, "` ", stringify!($ty), ".")] $variant),+
        }

        impl $ty {
            /// Returns the name this value has in profile output.
            #[must_use]
            $vis const fn name(self) -> &'static str {
                match self {
                    $($(#[cfg($cond)])? Self::$variant => $name),+
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

named!(
    /// Where a phase belongs in the summary.
    pub(crate) Category {
    Layout => "layout",
    Paint => "paint",
    Render => "render",
    A11y => "a11y",
    Reactive => "reactive",
    App => "app",
});

named!(
    /// A timed stage of a frame.
    pub Phase {
    Effect => "effect",
    Layout => "layout",
    LayoutSolve => "layout.solve",
    LayoutAttach => "layout.attach",
    LayoutSync => "layout.sync",
    LayoutSig => "layout.sig",
    LayoutDirty => "layout.dirty",
    LayoutChildren => "layout.children",
    LayoutPrepare => "layout.prepare",
    LayoutTranslate => "layout.translate",
    LayoutOverlays => "layout.overlays",
    LayoutSeed => "layout.seed",
    LayoutFinish => "layout.finish",
    HitRefresh => "hit.refresh",
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
    Gpu => "gpu",
    GpuWait => "gpu.wait",
    RenderPlan => "render.plan",
    RenderEncode => "render.encode",
    RenderRetain => "render.retain",
    RenderSubmit => "render.submit",
});

impl Phase {
    /// Returns the category of this phase.
    #[must_use]
    pub(crate) const fn category(self) -> Category {
        match self {
            Self::Layout
            | Self::LayoutSolve
            | Self::LayoutAttach
            | Self::LayoutSync
            | Self::LayoutSig
            | Self::LayoutDirty
            | Self::LayoutChildren
            | Self::LayoutPrepare
            | Self::LayoutTranslate
            | Self::LayoutOverlays
            | Self::LayoutSeed
            | Self::LayoutFinish => Category::Layout,
            Self::Emit | Self::Paint => Category::Paint,
            Self::RenderCpu
            | Self::RenderCompositePipeline
            | Self::Gpu
            | Self::GpuWait
            | Self::RenderPlan
            | Self::RenderEncode
            | Self::RenderRetain
            | Self::RenderSubmit => Category::Render,
            Self::A11y => Category::A11y,
            Self::Effect | Self::HitRefresh => Category::Reactive,
            Self::TuiPaint
            | Self::TuiRaster
            | Self::TuiWrite
            | Self::TuiPresent
            | Self::AppUpdate => Category::App,
        }
    }

    /// Returns whether this phase is not nested in another phase.
    #[must_use]
    pub const fn is_top_level(self) -> bool {
        matches!(
            self,
            Self::Effect | Self::Layout | Self::Emit | Self::Paint | Self::A11y
        )
    }
}

named!(
    /// Why a frame ran.
    #[derive(Default)] pub Cause {
    #[default]
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
    /// Returns the priority of this cause; a higher rank wins.
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

named!(
    /// A value that a frame counts.
    pub Counter {
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
    LayoutDirtyKernel => "layout.dirty.kernel",
    LayoutDirtySeed => "layout.dirty.seed",
    LayoutSolvedNodes => "layout.solved.n",
    EmitNodes => "emit.nodes",
    EmitReplays => "emit.replays",
    EmitCopied => "emit.copied",
    KernelMounted => "kernel.mounted",
    KernelPatched => "kernel.patched",
    KernelRemoved => "kernel.removed",
    TuiCellsRastered => "tui.cells_rastered",
    GpuPlanFull => "gpu.plan.full",
    GpuPlanRegions => "gpu.plan.regions",
    GpuPlanUnchanged => "gpu.plan.unchanged",
    GpuTimedPasses => "gpu.timed_passes",
    GpuBatches => "gpu.batches",
    TextAtlasMisses => "text.atlas.misses",
    TextAtlasCompactions => "text.atlas.compactions",
    TextOutlineMisses => "text.outline.misses",
});

named!(
    /// A cost that the profile attributes to a subject.
    kittest Hotspot {
    LayoutDirtyKernel => "layout.dirty.kernel",
    LayoutDirtySeed => "layout.dirty.seed",
    LayoutResized => "layout.resized",
    LayoutRoot => "layout.root",
    EmitResolved => "emit.resolved",
    GpuLayer if feature = "scene" => "gpu.layer",
    GpuFullRepaint if feature = "scene" => "gpu.full_repaint",
});

impl Hotspot {
    /// Returns the unit its counts are in.
    #[must_use]
    pub(crate) const fn unit(self) -> &'static str {
        match self {
            #[cfg(feature = "scene")]
            Self::GpuLayer => " px",
            _ => "",
        }
    }
}

named!(
    /// What a timed source line was doing.
    kittest SiteKind {
    Effect => "effect",
    Mount => "mount",
    Message => "message",
});

named!(
    /// A renderer backend.
    pub Backend {
    Gpu => "gpu",
    GpuSoftware => "gpu-software",
    Cpu => "cpu",
    Terminal => "terminal",
});

named!(
    /// Why the renderer fell back.
    pub FallbackCause {
    None => "none",
    NoAdapter => "no-adapter",
    DeviceError => "device-error",
    ForcedByEnv => "forced-by-env",
    UnknownEnv => "unknown-env",
});

/// The renderer backend of a frame and why it fell back, if it did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BackendStatus {
    /// The backend in use.
    pub(crate) kind: Backend,
    /// The reason for a fallback, if any.
    pub(crate) fallback: FallbackCause,
}

impl BackendStatus {
    /// Creates the status of `backend` after `fallback`.
    #[must_use]
    pub const fn new(backend: Backend, fallback: FallbackCause) -> Self {
        Self {
            kind: backend,
            fallback,
        }
    }

    /// The backend in use.
    #[must_use]
    pub const fn backend(&self) -> Backend {
        self.kind
    }

    /// The reason for a fallback, if any.
    #[must_use]
    pub const fn fallback(&self) -> FallbackCause {
        self.fallback
    }

    /// The status of the terminal host.
    pub const TERMINAL: Self = Self {
        kind: Backend::Terminal,
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

/// A phase and its time in nanoseconds.
pub(crate) type PhaseTime = (Phase, u64);
/// A counter and its value.
pub(crate) type CounterValue = (Counter, u64);

pub(crate) fn accumulate<K: Copy + PartialEq>(list: &mut Vec<(K, u64)>, key: K, add: u64) {
    if let Some((_, ns)) = list.iter_mut().find(|(k, _)| *k == key) {
        *ns = ns.saturating_add(add);
    } else {
        list.push((key, add));
    }
}
