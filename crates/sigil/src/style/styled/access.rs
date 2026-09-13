use crate::style::Style;

pub trait StyleAccess: Sized {
    fn style_mut(&mut self) -> &mut Style;
}
