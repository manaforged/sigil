use core::error::Error;

use crate::hal::geometry::{Color, PhysicalSize};

pub trait Renderer {
    type Error: Error + Send + Sync + 'static;

    type Surface;

    type WindowHandle;

    fn create_surface(
        &mut self,
        window: Self::WindowHandle,
        initial_size: PhysicalSize,
    ) -> Result<Self::Surface, Self::Error>;

    fn resize(
        &mut self,
        surface: &mut Self::Surface,
        size: PhysicalSize,
    ) -> Result<(), Self::Error>;

    fn clear(&mut self, surface: &mut Self::Surface, color: Color) -> Result<(), Self::Error>;
}
