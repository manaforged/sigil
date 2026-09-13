use core::fmt;

#[derive(Debug)]
#[non_exhaustive]
pub enum RendererError {
    NoAdapter,
    DeviceRequest(wgpu::RequestDeviceError),
    SurfaceCreate(wgpu::CreateSurfaceError),
    NoSupportedFormat,
    SurfaceValidation,
    NotInitialized,
    InvalidViewProj,
    InvalidViewport,
    Layer(crate::renderer::layer_pool::LayerError),
    Software(String),
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
            Self::Software(detail) => write!(f, "software renderer failed: {detail}"),
            Self::NotInitialized => {
                f.write_str("renderer has not yet picked a GPU device; call create_surface first")
            }
            Self::InvalidViewProj => {
                f.write_str("view_proj matrix contains a non-finite value (NaN or infinity)")
            }
            Self::InvalidViewport => {
                f.write_str("viewport dimensions must be strictly positive (width > 0, height > 0)")
            }
            Self::Layer(crate::renderer::layer_pool::LayerError::Empty) => {
                f.write_str("offscreen layer extent is zero")
            }
            Self::Layer(crate::renderer::layer_pool::LayerError::TooLarge { requested, max }) => {
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
            | Self::Layer(_)
            | Self::Software(_) => None,
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
