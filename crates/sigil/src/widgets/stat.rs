use std::rc::Rc;

use crate::kernel::{
    Cx, IntoNode, NodeBuilder, NodeId, Role, TextBuilder, TextContent, container, text,
};
use crate::layout::{LayoutBuilderExt, LayoutStyle};
use crate::reactive::Reactive;
use crate::style::{StyleAtom, Token, Tone, space};

use super::patterns::card;
use super::sparkline::sparkline;

#[must_use]
pub struct StatBuilder {
    label: TextContent,
    value: TextContent,
    delta: Option<(TextContent, Tone)>,
    caption: Option<TextContent>,
    trend: Option<Rc<dyn Reactive<Vec<f32>>>>,
    range: Option<(f32, f32)>,
    tone: Option<Tone>,
}

pub fn stat(label: impl Into<TextContent>, value: impl Into<TextContent>) -> StatBuilder {
    StatBuilder {
        label: label.into(),
        value: value.into(),
        delta: None,
        caption: None,
        trend: None,
        range: None,
        tone: None,
    }
}

impl StatBuilder {
    pub fn delta(mut self, change: impl Into<TextContent>, tone: Tone) -> Self {
        self.delta = Some((change.into(), tone));
        self
    }

    pub fn caption(mut self, caption: impl Into<TextContent>) -> Self {
        self.caption = Some(caption.into());
        self
    }

    pub fn trend(mut self, values: impl Reactive<Vec<f32>>) -> Self {
        self.trend = Some(Rc::new(values));
        self
    }

    pub const fn range(mut self, low: f32, high: f32) -> Self {
        self.range = Some((low, high));
        self
    }

    pub const fn tone(mut self, tone: Tone) -> Self {
        self.tone = Some(tone);
        self
    }

    #[must_use]
    pub fn into_node_builder(self) -> NodeBuilder {
        let name = self.label.as_static_str().unwrap_or("Statistic").to_owned();
        let value = text(self.value)
            .nowrap()
            .ellipsize()
            .class("text-xl font-semibold tabular-nums");
        let value = match self.tone {
            Some(tone) => value.class(tone.solid().text()),
            None => value.class(Token::OnSurface.text()),
        };
        let mut body = container()
            .role(Role::Group)
            .label(name)
            .layout(LayoutStyle::flex_column().fill_width().gap(space(1)))
            .child(caption_text(
                self.label,
                "text-sm",
                Token::OnSurfaceMuted.text(),
            ))
            .child(value);
        if let Some((change, tone)) = self.delta {
            body = body.child(caption_text(
                change,
                "text-xs tabular-nums",
                tone.solid().text(),
            ));
        }
        if let Some(caption) = self.caption {
            body = body.child(caption_text(
                caption,
                "text-xs",
                Token::OnSurfaceMuted.text(),
            ));
        }
        if let Some(values) = self.trend {
            let line = sparkline(move || values.read()).tone(self.tone.unwrap_or(Tone::Primary));
            body = body.child(match self.range {
                Some((low, high)) => line.range(low, high),
                None => line,
            });
        }
        card(body).into_node_builder()
    }
}

fn caption_text(content: TextContent, size: &'static str, ink: StyleAtom) -> TextBuilder {
    text(content).nowrap().ellipsize().class(size).class(ink)
}

impl From<StatBuilder> for NodeBuilder {
    fn from(stat: StatBuilder) -> Self {
        stat.into_node_builder()
    }
}

impl IntoNode for StatBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.into_node_builder().build(cx)
    }
}
