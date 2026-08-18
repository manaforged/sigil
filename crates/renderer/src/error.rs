//! Error type for the wgpu-backed renderer.
//!
//! Hand-rolled so the renderer keeps only `wgpu + pollster +
//! raw-window-handle` as runtime deps — no `thiserror`. The surface is
//! small enough that the boilerplate is bounded.

use core::fmt;

/// Errors returned by [`WgpuRenderer`][crate::WgpuRenderer] and its
/// surface operations.
///
/// Transient per-frame conditions (`Lost`, `Outdated`, `Timeout`,
/// `Occluded`) are recovered internally by reconfiguring the surface
/// and skipping the frame; they never surface as `RendererError`.
#[derive(Debug)]
#[non_exhaustive]
pub enum RendererError {
    /// No compatible GPU adapter was available.
    NoAdapter,
    /// The GPU refused to hand out a device matching our limits.
    DeviceRequest(wgpu::RequestDeviceError),
    /// The backend could not build a surface for the provided window.
    SurfaceCreate(wgpu::CreateSurfaceError),
    /// The adapter advertised no usable texture formats for this
    /// surface. Distinct from `NoAdapter` — we picked a GPU, but it
    /// cannot present to this window with any format we know how to
    /// drive.
    NoSupportedFormat,
    /// The surface reported an unrecoverable validation error during
    /// acquisition. Distinct from `Outdated`/`Lost`, which we recover.
    SurfaceValidation,
    /// `resize` or `clear` was called before the first successful
    /// `create_surface`, so there is no device/queue yet. A bug in the
    /// caller rather than a hardware failure.
    NotInitialized,
    /// A `view_proj` matrix passed to `submit_world` contained a non-finite
    /// value (NaN or infinity). The matrix must be fully finite before GPU
    /// upload.
    InvalidViewProj,
    /// A viewport dimension passed to `submit_screen` or `submit_world` was
    /// zero or negative. Both width and height must be strictly positive.
    InvalidViewport,
    /// An offscreen layer could not be allocated.
    Layer(crate::layer_pool::LayerError),
}

impl fmt::Display for RendererError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoAdapter => f.write_str("no compatible GPU adapter found"),
            Self::DeviceRequest(e) => write!(f, "failed to request GPU device: {e}"),
            Self::SurfaceCreate(e) => write!(f, "failed to create GPU surface: {e}"),
            Self::NoSupportedFormat => {
                f.write_str("GPU adapter reported no usable swapchain formats for this surface")
            }
            Self::SurfaceValidation => {
                f.write_str("surface acquisition failed with a validation error")
            }
            Self::NotInitialized => {
                f.write_str("renderer has not yet picked a GPU device; call create_surface first")
            }
            Self::InvalidViewProj => {
                f.write_str("view_proj matrix contains a non-finite value (NaN or infinity)")
            }
            Self::InvalidViewport => {
                f.write_str("viewport dimensions must be strictly positive (width > 0, height > 0)")
            }
            Self::Layer(crate::layer_pool::LayerError::Empty) => {
                f.write_str("offscreen layer extent is zero")
            }
            Self::Layer(crate::layer_pool::LayerError::TooLarge { requested, max }) => {
                write!(
                    f,
                    "offscreen layer {requested}px exceeds device limit {max}px"
                )
            }
        }
    }
}

impl core::error::Error for RendererError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::DeviceRequest(e) => Some(e),
            Self::SurfaceCreate(e) => Some(e),
            Self::NoAdapter
            | Self::NoSupportedFormat
            | Self::SurfaceValidation
            | Self::NotInitialized
            | Self::InvalidViewProj
            | Self::InvalidViewport
            | Self::Layer(_) => None,
        }
    }
}

impl From<wgpu::RequestDeviceError> for RendererError {
    fn from(value: wgpu::RequestDeviceError) -> Self {
        Self::DeviceRequest(value)
    }
}

impl From<wgpu::CreateSurfaceError> for RendererError {
    fn from(value: wgpu::CreateSurfaceError) -> Self {
        Self::SurfaceCreate(value)
    }
}
