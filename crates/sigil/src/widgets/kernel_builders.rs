use crate::kernel::{ImageBuilder, TextBuilder};

builder_impls!([] TextBuilder: kernel);
builder_impls!([] ImageBuilder: kernel);

impl crate::style::States for TextBuilder {
    crate::style::state_methods!(forward node);
}
