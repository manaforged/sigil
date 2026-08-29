use crate::Style;

pub trait StyleAccess: Sized {
    fn style_mut(&mut self) -> &mut Style;
}
