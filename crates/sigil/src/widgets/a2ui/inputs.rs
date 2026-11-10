use crate::layout::Space;
use serde_json::Value;

use crate::kernel::{IntoNode, NodeBuilder, container, fragment};
use crate::layout::LayoutStyle;
use crate::reactive::{Signal, effect, signal, untrack};
use crate::widgets::{Date, Layout, checkbox, date_field, field, radio_group, slider, text_field};

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

fn first(value: &Value) -> String {
    selected(value).into_iter().next().unwrap_or_default()
}

fn iso_date(raw: &str) -> Option<Date> {
    let mut parts = raw.get(..10)?.splitn(3, '-');
    let year = parts.next()?.parse().ok()?;
    let month = parts.next()?.parse().ok()?;
    let day = parts.next()?.parse().ok()?;
    Date::new(year, month, day)
}

fn iso(day: Date) -> String {
    format!("{:04}-{:02}-{:02}", day.year, day.month, day.day)
}

fn typed(kind: FieldKind, text: &str) -> Value {
    let number = text
        .parse::<f64>()
        .ok()
        .and_then(serde_json::Number::from_f64);
    match (kind, number) {
        (FieldKind::Number, Some(number)) => Value::Number(number),
        _ => Value::String(text.to_string()),
    }
}

fn level(value: &Value, min: f32, max: f32) -> f32 {
    #[expect(clippy::cast_possible_truncation, reason = "slider values fit f32")]
    let level = value.as_f64().map_or(min, |n| n as f32);
    level.clamp(min, max)
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

    fn mirror<T: PartialEq + 'static>(
        &self,
        value: &Dynamic,
        local: Signal<T>,
        read: impl Fn(&T, &Value) -> T + 'static,
    ) {
        let (Some(path), model) = (self.bound(value), self.model) else {
            return;
        };
        effect(move || {
            let current = model.get(&path);
            let next = untrack(|| local.with(|now| read(now, &current)));
            untrack(|| local.set(next));
        });
    }

    fn labelled(&self, label: &Dynamic, control: impl IntoNode + 'static) -> NodeBuilder {
        let label = self.shown(label);
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
        let text = signal(self.shown(value));
        self.mirror(value, text, move |now: &String, current| {
            if typed(kind, now) == *current {
                now.clone()
            } else {
                display(current)
            }
        });
        let (render, path) = (self.clone(), self.bound(value));
        let mut control = text_field(text)
            .change(move |entered: &str| render.write(path.as_ref(), typed(kind, entered)));
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
        self.mirror(value, on, |_, current| current.as_bool().unwrap_or(false));
        let (render, path) = (self.clone(), self.bound(value));
        checkbox(on)
            .label(self.shown(label))
            .change(move |on| render.write(path.as_ref(), Value::Bool(on)))
            .into()
    }

    fn slide(&self, label: &Dynamic, value: &Dynamic, min: f32, max: f32) -> NodeBuilder {
        let max = if max > min { max } else { min + 1.0 };
        let at = signal(level(&self.value(value), min, max));
        self.mirror(value, at, move |_, current| level(current, min, max));
        let (render, path) = (self.clone(), self.bound(value));
        slider(at, min..=max)
            .label(self.shown(label))
            .change(move |level| {
                let number = serde_json::Number::from_f64(f64::from(level));
                render.write(path.as_ref(), number.map_or(Value::Null, Value::Number));
            })
            .into()
    }

    fn picker(&self, picker: &Picker) -> NodeBuilder {
        if picker.multiple {
            return self.many(picker);
        }
        let pick = signal(first(&self.value(&picker.value)));
        self.mirror(&picker.value, pick, |_, current| first(current));
        let mut group = radio_group(pick).label(self.shown(&picker.label));
        for option in &picker.options {
            group = group.option(option.value.clone(), self.shown(&option.label));
        }
        let (render, path) = (self.clone(), self.bound(&picker.value));
        effect(move || {
            let value = pick.get();
            let Some(path) = path.as_ref() else { return };
            if !value.is_empty() && untrack(|| first(&render.model.peek(path))) != value {
                untrack(|| render.write(Some(path), Value::Array(vec![Value::String(value)])));
            }
        });
        self.labelled(&picker.label, group)
    }

    fn many(&self, picker: &Picker) -> NodeBuilder {
        let chosen = selected(&self.value(&picker.value));
        let path = self.bound(&picker.value);
        let mut column = container().layout(LayoutStyle::flex_column().gap(Space::Xs));
        for option in &picker.options {
            let on = signal(chosen.contains(&option.value));
            let wanted = option.value.clone();
            self.mirror(&picker.value, on, move |_, current| {
                selected(current).contains(&wanted)
            });
            let (render, path, key) = (self.clone(), path.clone(), option.value.clone());
            let control = checkbox(on)
                .label(self.shown(&option.label))
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
        self.mirror(&clock.value, day, |now, current| {
            iso_date(&display(current)).unwrap_or(*now)
        });
        let (render, path) = (self.clone(), self.bound(&clock.value));
        effect(move || {
            let day = day.get();
            let Some(path) = path.as_ref() else { return };
            let stored = untrack(|| iso_date(&display(&render.model.peek(path))));
            if stored != Some(day) {
                untrack(|| render.write(Some(path), Value::String(iso(day))));
            }
        });
        self.labelled(&clock.label, date_field(day))
    }
}
