use serde_json::Value;

use crate::kernel::{IntoNode, NodeBuilder, container, fragment};
use crate::layout::LayoutStyle;
use crate::reactive::{effect, signal, untrack};
use crate::style::space;
use crate::widgets::{Date, Widget, checkbox, date_field, field, radio_group, slider, text_field};

use super::model::display;
use super::render::{Render, note};
use super::spec::{Clock, Dynamic, FieldKind, Kind, Picker};

fn selected(value: &Value) -> Vec<String> {
    match value {
        Value::Array(items) => items.iter().map(display).collect(),
        Value::Null => Vec::new(),
        other => vec![display(other)],
    }
}

fn iso_date(raw: &str) -> Option<Date> {
    let mut parts = raw.get(..10)?.splitn(3, '-');
    let year = parts.next()?.parse().ok()?;
    let month = parts.next()?.parse().ok()?;
    let day = parts.next()?.parse().ok()?;
    Date::new(year, month, day)
}

impl Render {
    pub(super) fn input(&self, kind: &Kind) -> NodeBuilder {
        match kind {
            Kind::TextField { label, value, kind } => self.text_input(label, value, *kind, ""),
            Kind::CheckBox { label, value } => self.check(label, value),
            Kind::ChoicePicker(picker) => self.picker(picker),
            Kind::Slider {
                label,
                value,
                min,
                max,
            } => self.slide(label, value, *min, *max),
            Kind::DateTime(clock) => self.clock(clock),
            _ => note("Unsupported component."),
        }
    }

    fn write(&self, path: Option<&String>, value: Value) {
        if let Some(path) = path {
            self.model.set(path, value);
        }
    }

    fn labelled(&self, label: &Dynamic, control: impl IntoNode + 'static) -> NodeBuilder {
        let label = display(&self.value(label));
        if label.is_empty() {
            return container()
                .layout(LayoutStyle::block().fill_width())
                .child(control);
        }
        fragment().child(field(label, control))
    }

    fn text_input(
        &self,
        label: &Dynamic,
        value: &Dynamic,
        kind: FieldKind,
        hint: &str,
    ) -> NodeBuilder {
        let text = signal(display(&self.value(value)));
        let (render, path) = (self.clone(), self.bound(value));
        let mut control = text_field(text).change(move |typed: &str| {
            let number = typed
                .parse::<f64>()
                .ok()
                .and_then(serde_json::Number::from_f64);
            let written = match (kind, number) {
                (FieldKind::Number, Some(number)) => Value::Number(number),
                _ => Value::String(typed.to_string()),
            };
            render.write(path.as_ref(), written);
        });
        if matches!(kind, FieldKind::Obscured) {
            control = control.password();
        }
        if !hint.is_empty() {
            control = control.placeholder(hint.to_string());
        }
        self.labelled(label, control)
    }

    fn check(&self, label: &Dynamic, value: &Dynamic) -> NodeBuilder {
        let on = signal(self.value(value).as_bool().unwrap_or(false));
        let (render, path) = (self.clone(), self.bound(value));
        checkbox(on)
            .label(display(&self.value(label)))
            .change(move |on| render.write(path.as_ref(), Value::Bool(on)))
            .into()
    }

    fn slide(&self, label: &Dynamic, value: &Dynamic, min: f32, max: f32) -> NodeBuilder {
        let max = if max > min { max } else { min + 1.0 };
        #[expect(clippy::cast_possible_truncation, reason = "slider values fit f32")]
        let start = self.value(value).as_f64().map_or(min, |n| n as f32);
        let level = signal(start.clamp(min, max));
        let (render, path) = (self.clone(), self.bound(value));
        slider(level, min..=max)
            .label(display(&self.value(label)))
            .change(move |level| {
                let number = serde_json::Number::from_f64(f64::from(level));
                render.write(path.as_ref(), number.map_or(Value::Null, Value::Number));
            })
            .into()
    }

    fn picker(&self, picker: &Picker) -> NodeBuilder {
        let chosen = selected(&self.value(&picker.value));
        if picker.multiple {
            return self.many(picker, &chosen);
        }
        let pick = signal(chosen.first().cloned().unwrap_or_default());
        let mut group = radio_group(pick).label(display(&self.value(&picker.label)));
        for option in &picker.options {
            group = group.option(option.value.clone(), display(&self.value(&option.label)));
        }
        let (render, path) = (self.clone(), self.bound(&picker.value));
        effect(move || {
            let value = pick.get();
            if !value.is_empty() {
                untrack(|| render.write(path.as_ref(), Value::Array(vec![Value::String(value)])));
            }
        });
        self.labelled(&picker.label, group)
    }

    fn many(&self, picker: &Picker, chosen: &[String]) -> NodeBuilder {
        let path = self.bound(&picker.value);
        let mut column = container().layout(LayoutStyle::flex_column().gap(space(1)));
        for option in &picker.options {
            let on = signal(chosen.contains(&option.value));
            let (render, path, key) = (self.clone(), path.clone(), option.value.clone());
            let control = checkbox(on)
                .label(display(&self.value(&option.label)))
                .change(move |on| {
                    let Some(path) = path.as_ref() else { return };
                    let mut list = selected(&render.model.peek(path));
                    list.retain(|item| *item != key);
                    if on {
                        list.push(key.clone());
                    }
                    render.write(Some(path), Value::from(list));
                });
            column = column.child(control);
        }
        self.labelled(&picker.label, column)
    }

    fn clock(&self, clock: &Clock) -> NodeBuilder {
        let raw = display(&self.value(&clock.value));
        let date = iso_date(&raw).filter(|_| clock.date && !clock.time);
        let Some(date) = date else {
            let hint = match (clock.date, clock.time) {
                (true, true) => "YYYY-MM-DDTHH:MM",
                (false, true) => "HH:MM",
                _ => "YYYY-MM-DD",
            };
            return self.text_input(&clock.label, &clock.value, FieldKind::Short, hint);
        };
        let day = signal(date);
        let (render, path) = (self.clone(), self.bound(&clock.value));
        effect(move || {
            let day = day.get();
            let iso = format!("{:04}-{:02}-{:02}", day.year, day.month, day.day);
            untrack(|| render.write(path.as_ref(), Value::String(iso)));
        });
        self.labelled(&clock.label, date_field(day))
    }
}
