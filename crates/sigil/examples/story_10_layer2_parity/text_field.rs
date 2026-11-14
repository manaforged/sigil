use std::fmt;
use std::rc::Rc;

use sigil::animation::Transition;
use sigil::clipboard;
use sigil::events::{A11yEdit, DragPhase, ImeEvent, Key, KeyEvent, KeyPhase, Modifiers, NamedKey};
use sigil::layout::{LayoutStyle, Overflow, Position};
use sigil::node::{Cx, NodeId};
use sigil::prelude::*;
use sigil::style::{BORDER_WIDTH, Metrics};
use sigil::text::{CaretGeometry, HitPosition, TextStyle, measure_text};
use sigil::widgets::{Framed, WidgetFrame};

type TextCallback = Rc<dyn Fn(&str)>;

fn read_sig<T: Clone + 'static>(sig: Signal<T>, peek: bool) -> T {
    if peek { sig.peek() } else { sig.get() }
}

fn caret_at(text: &str, style: &TextStyle, byte: usize) -> Option<CaretGeometry> {
    measure_text(|e| e.caret(text, style, None, HitPosition::downstream(byte)))
}

#[derive(Clone)]
struct FieldRuntime {
    state: TextState,
    disabled: Option<Signal<bool>>,
    change: TextCallback,
    submit: TextCallback,
    clipboard: Rc<dyn clipboard::Clipboard>,
    clip_fail: Signal<bool>,
    text_style: Signal<TextStyle>,
    scroll_x: Signal<f32>,
    column: f32,
    focused: Signal<bool>,
    preedit: Signal<String>,
    preedit_cursor: Signal<Option<(usize, usize)>>,
}

impl FieldRuntime {
    fn is_disabled(&self) -> bool {
        self.disabled.is_some_and(|flag| flag.peek())
    }
    fn edit(&self, op: impl FnOnce(&TextState)) {
        let before = self.state.value().peek();
        op(&self.state);
        self.keep_caret_visible();
        let after = self.state.value().peek();
        if after != before {
            (self.change)(after.as_str());
        }
    }
    fn copy_selection(&self) -> bool {
        let Some(selected) = self.state.selected_text() else {
            return false;
        };
        let copied = self.clipboard.set(&selected).is_ok();
        self.clip_fail.set(!copied);
        copied
    }
    fn composed(&self, peek: bool) -> (String, usize) {
        let text = read_sig(self.state.value(), peek);
        let caret = read_sig(self.state.caret(), peek);
        let preedit = read_sig(self.preedit, peek);
        let cursor = read_sig(self.preedit_cursor, peek);
        if preedit.is_empty() {
            return (text.clone(), caret.min(text.len()));
        }
        let offset = cursor.map_or(preedit.len(), |(_, end)| end.min(preedit.len()));
        (
            TextState::splice_preedit(&text, caret, &preedit),
            caret.min(text.len()) + offset,
        )
    }
    fn keep_caret_visible(&self) {
        let (composed, byte) = self.composed(true);
        let Some(pos) = caret_at(&composed, &self.text_style.peek(), byte) else {
            return;
        };
        let mut scroll = self.scroll_x.peek();
        if pos.x() < scroll {
            scroll = pos.x();
        } else if pos.x() > scroll + self.column {
            scroll = pos.x() - self.column;
        }
        self.scroll_x.set(scroll.max(0.0));
    }
    fn caret_geometry(&self) -> Option<CaretGeometry> {
        if self.disabled.is_some_and(|flag| flag.get()) {
            return None;
        }
        let (composed, byte) = self.composed(false);
        let pos = caret_at(&composed, &self.text_style.get(), byte)?;
        Some(CaretGeometry::new(
            (pos.x() - self.scroll_x.get().max(0.0)).max(0.0),
            pos.top(),
            pos.height(),
        ))
    }
    fn hit(&self, x: f32) -> usize {
        let (value, style) = (self.state.value().peek(), self.text_style.peek());
        let point = Point::new(x + self.scroll_x.peek(), 0.0);
        measure_text(|e| e.hit(&value, &style, None, point)).map_or(0, |h| h.byte_index())
    }
    fn insert_single_line(&self, raw: &str) {
        let clean: String = raw
            .chars()
            .filter(|ch| *ch != '\n' && *ch != '\r')
            .collect();
        self.edit(|state| state.insert(&clean));
    }
    fn clear_preedit(&self) {
        if !self.preedit.peek().is_empty() {
            self.preedit.set(String::new());
        }
        if self.preedit_cursor.peek().is_some() {
            self.preedit_cursor.set(None);
        }
    }
    fn apply_ime(&self, event: &ImeEvent) {
        if matches!(event, ImeEvent::Disabled) {
            self.clear_preedit();
            return;
        }
        if self.is_disabled() {
            return;
        }
        match event {
            ImeEvent::Preedit(text, range) => {
                if text.is_empty() {
                    self.clear_preedit();
                } else {
                    self.preedit.set(text.clone());
                    self.preedit_cursor.set(*range);
                }
            }
            ImeEvent::Commit(text) => {
                self.clear_preedit();
                self.insert_single_line(text);
            }
            _ => {}
        }
    }
    fn apply_a11y(&self, edit: &A11yEdit) -> bool {
        if self.is_disabled() {
            return true;
        }
        match edit {
            A11yEdit::Text(text) => {
                let clean: String = text.chars().filter(|ch| !ch.is_control()).collect();
                self.edit(|state| state.replace(&clean));
                true
            }
            A11yEdit::Select { anchor, focus } => {
                self.state.select(*anchor);
                self.state.extend(*focus);
                self.keep_caret_visible();
                true
            }
            _ => false,
        }
    }
    fn move_edge(&self, end: bool, extend: bool) {
        let pos = if end {
            self.state.value().peek().len()
        } else {
            0
        };
        if extend {
            self.state.extend(pos);
        } else {
            self.state.select(pos);
        }
    }
}

/// External reimplementation of `text_field` using only public Layer 1 and Layer 2 APIs.
#[must_use]
pub struct CustomTextFieldBuilder {
    state: TextState,
    placeholder: TextContent,
    disabled: Option<Signal<bool>>,
    invalid: Option<Signal<bool>>,
    width: Option<f32>,
    on_change: Option<TextCallback>,
    on_submit: Option<TextCallback>,
    clipboard: Option<Rc<dyn clipboard::Clipboard>>,
    frame: WidgetFrame,
}

fn chain(first: Option<TextCallback>, next: impl Fn(&str) + 'static) -> TextCallback {
    let Some(first) = first else {
        return Rc::new(next);
    };
    Rc::new(move |text: &str| {
        first(text);
        next(text);
    })
}

/// Creates a custom single-line text field bound to `value`.
pub fn custom_text_field(value: impl Into<TextState>) -> CustomTextFieldBuilder {
    CustomTextFieldBuilder {
        state: value.into(),
        placeholder: TextContent::Static(String::new()),
        disabled: None,
        invalid: None,
        width: None,
        on_change: None,
        on_submit: None,
        clipboard: None,
        frame: WidgetFrame::new(),
    }
}

impl fmt::Debug for CustomTextFieldBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CustomTextFieldBuilder")
            .finish_non_exhaustive()
    }
}

impl Framed for CustomTextFieldBuilder {
    fn frame_mut(&mut self) -> &mut WidgetFrame {
        &mut self.frame
    }
}

impl IntoNode for CustomTextFieldBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.root().build(cx)
    }
}

impl CustomTextFieldBuilder {
    /// Sets the placeholder text shown while the field is empty.
    pub fn placeholder(mut self, text: impl Into<TextContent>) -> Self {
        self.placeholder = text.into();
        self
    }

    /// Disables editing while `disabled` is true.
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.disabled = Some(disabled.into_signal());
        self
    }

    /// Marks the field invalid while `invalid` is true; each call adds a condition.
    pub fn invalid(mut self, invalid: impl Reactive<bool>) -> Self {
        let added = invalid.into_signal();
        self.invalid = Some(match self.invalid {
            Some(before) => memo(move || before.get() || added.get()),
            None => added,
        });
        self
    }

    /// Sets the width of the field in logical pixels.
    pub const fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Calls `change` after each edit that changes the text; each call adds a callback.
    pub fn change(mut self, change: impl Fn(&str) + 'static) -> Self {
        self.on_change = Some(chain(self.on_change.take(), change));
        self
    }

    /// Calls `submit` when Enter is pressed inside the field; each call adds a callback.
    pub fn submit(mut self, submit: impl Fn(&str) + 'static) -> Self {
        self.on_submit = Some(chain(self.on_submit.take(), submit));
        self
    }

    /// Replaces the installed clipboard that cut, copy, and paste use.
    pub fn clipboard(mut self, clipboard: impl clipboard::Clipboard + 'static) -> Self {
        self.clipboard = Some(Rc::new(clipboard));
        self
    }

    fn root(mut self) -> NodeBuilder {
        let rt = FieldRuntime {
            state: self.state,
            disabled: self.disabled,
            change: self.on_change.take().unwrap_or_else(|| Rc::new(|_| {})),
            submit: self.on_submit.take().unwrap_or_else(|| Rc::new(|_| {})),
            clipboard: self.clipboard.take().unwrap_or_else(clipboard::current),
            clip_fail: signal(false),
            text_style: signal(TextStyle::default()),
            scroll_x: signal(0.0),
            column: self.width.map_or(156.0, |w| (w - 26.0).max(0.0)),
            focused: signal(false),
            preedit: signal(String::new()),
            preedit_cursor: signal(None),
        };
        let ph_str = self.placeholder.resolve();
        let label = if ph_str.is_empty() {
            String::from("Text field")
        } else {
            ph_str.clone()
        };
        let (anchor, displayed) = displayed_field_text(&rt, self.placeholder);
        let (invalid_flag, clip_fail) = (self.invalid, rt.clip_fail);
        let invalid = memo(move || invalid_flag.is_some_and(Signal::get) || clip_fail.get());
        let node = paint_custom_chrome(
            container()
                .focusable(true)
                .track_focused(rt.focused)
                .capture_text_style(rt.text_style)
                .role(Role::TextInput)
                .accessible(label)
                .placeholder(ph_str)
                .invalid(invalid)
                .text_value(rt.state.value())
                .text_cursor(rt.state.caret(), anchor)
                .layout(field_layout(self.width, false))
                .child(
                    container()
                        .layout(field_layout(self.width, true))
                        .child(field_text_region(rt.clone(), displayed)),
                ),
            rt.focused,
            rt.disabled,
            invalid,
        );
        self.frame.finish(attach_field_events(node, &rt))
    }
}

fn displayed_field_text(
    rt: &FieldRuntime,
    placeholder: TextContent,
) -> (Signal<Option<usize>>, Signal<String>) {
    let state_ref = rt.state.clone();
    let anchor = memo(move || {
        let caret = state_ref.caret().get();
        state_ref
            .selection()
            .map(|r| if r.start == caret { r.end } else { r.start })
    });
    let (val_sig, pre_sig, caret_sig) = (rt.state.value(), rt.preedit, rt.state.caret());
    let displayed = memo(move || {
        let (current, composing) = (val_sig.get(), pre_sig.get());
        if !composing.is_empty() {
            TextState::splice_preedit(&current, caret_sig.get(), &composing)
        } else if current.is_empty() {
            placeholder.resolve()
        } else {
            current
        }
    });
    (anchor, displayed)
}

fn paint_custom_chrome(
    node: NodeBuilder,
    focused: Signal<bool>,
    disabled: Option<Signal<bool>>,
    invalid: Signal<bool>,
) -> NodeBuilder {
    let (hovered, pressed) = (signal(false), signal(false));
    let hover_border = memo(move || {
        !disabled.is_some_and(Signal::get)
            && !invalid.get()
            && hovered.get()
            && !focused.get()
            && !pressed.get()
    });
    let active_border = memo(move || {
        !disabled.is_some_and(Signal::get) && !invalid.get() && (focused.get() || pressed.get())
    });
    let node = node
        .track_hovered(hovered)
        .track_pressed(pressed)
        .radius(Radius::Md)
        .background(Token::Field)
        .border_width(BORDER_WIDTH)
        .border_color(Tone::Neutral.step(7))
        .transition(Transition::smooth())
        .paint_when(hover_border, |s| s.border_color(Tone::Neutral.step(8)))
        .paint_when(active_border, |s| s.border_color(Token::Accent));
    match disabled {
        Some(flag) => node
            .disabled(flag)
            .paint_when(memo(move || !flag.get() && invalid.get()), |s| {
                s.border_color(Color::rgb(211, 47, 47))
            })
            .paint_when(flag, |s| s.border_color(Tone::Neutral.step(6)).opacity(0.5)),
        None => node.paint_when(invalid, |s| s.border_color(Color::rgb(211, 47, 47))),
    }
}

fn field_layout(width: Option<f32>, inner: bool) -> LayoutStyle {
    let m = Metrics::current();
    if !inner {
        let base = LayoutStyle::flex_row().min_size(
            width.map_or(96.0, |_| 0.0),
            m.control_height(ControlSize::Default),
        );
        return match width {
            Some(w) => base.width(w),
            None => base.fill_width(),
        }
        .align_items(AlignItems::Stretch);
    }
    let inset = if m.is_grid() { m.icon_size() } else { 1.0 };
    let v_pad = if m.is_grid() { Space::None } else { Space::Xs };
    let base = width.map_or_else(
        || LayoutStyle::flex_row().fill_width().grow(1.0),
        |w| LayoutStyle::flex_row().width(2.0_f32.mul_add(-inset, w).max(0.0)),
    );
    base.align_items(AlignItems::Center)
        .padding_x(Space::Md)
        .padding_y(v_pad)
}

fn abs_box(x: f32, top: f32, size: Option<(f32, f32)>) -> NodeBuilder {
    let base = LayoutStyle::block()
        .position(Position::Absolute)
        .inset_left(x)
        .inset_top(top);
    container().hittable(false).layout(match size {
        Some((w, h)) => base.fixed_size(w, h),
        None => base,
    })
}

fn field_text_region(rt: FieldRuntime, displayed: Signal<String>) -> NodeBuilder {
    let scroll = rt.scroll_x;
    let track_pos = memo(move || Point::new(-scroll.get(), 0.0));
    let value = rt.state.value();
    let preedit = rt.preedit;
    let placeholder_visible = memo(move || value.get().is_empty() && preedit.get().is_empty());
    let (sel_rt, caret_rt) = (rt.clone(), rt.clone());
    let track = container()
        .hittable(false)
        .layout(LayoutStyle::block())
        .offset(track_pos)
        .child(abs_box(0.0, 0.0, None).children_reactive(move || field_selection_rects(&sel_rt)))
        .child(abs_box(0.0, 0.0, None).children_reactive(move || field_caret_rects(&caret_rt)))
        .child(
            text(displayed)
                .hittable(false)
                .layout(LayoutStyle::block())
                .text_color(Token::OnSurface)
                .paint_when(placeholder_visible, |s| s.text_color(Token::OnSurfaceMuted)),
        );
    container()
        .layout(
            LayoutStyle::block()
                .position(Position::Relative)
                .fill_width()
                .overflow_x(Overflow::Hidden),
        )
        .child(
            text("\u{200b}")
                .hittable(false)
                .layout(LayoutStyle::block()),
        )
        .child(track)
        .pointer_drag(move |phase, event| match phase {
            DragPhase::Start if rt.is_disabled() => event.prevent_default(),
            DragPhase::Start => {
                rt.state.select(rt.hit(event.position().x));
                rt.keep_caret_visible();
            }
            DragPhase::Move if !rt.is_disabled() => {
                rt.state.extend(rt.hit(event.position().x));
                rt.keep_caret_visible();
                event.stop_propagation();
            }
            _ => {}
        })
}

fn field_selection_rects(rt: &FieldRuntime) -> Vec<NodeBuilder> {
    let Some(range) = rt.state.selection() else {
        return Vec::new();
    };
    let (value, style) = (rt.state.value().get(), rt.text_style.get());
    let Some((s, e)) = caret_at(&value, &style, range.start)
        .zip(caret_at(&value, &style, range.end))
        .filter(|(s, e)| e.x() > s.x())
    else {
        return Vec::new();
    };
    vec![abs_box(s.x(), s.top(), Some((e.x() - s.x(), s.height()))).background(Token::Selection)]
}

fn field_caret_rects(rt: &FieldRuntime) -> Vec<NodeBuilder> {
    if !rt.focused.get() {
        return Vec::new();
    }
    let (composed, byte) = rt.composed(false);
    let Some(p) = caret_at(&composed, &rt.text_style.get(), byte) else {
        return Vec::new();
    };
    vec![abs_box(p.x(), p.top(), Some((2.0, p.height()))).background(Token::OnSurface)]
}

fn handle_named_key(key: NamedKey, mods: Modifiers, rt: &FieldRuntime) -> bool {
    let shift = mods.contains(Modifiers::SHIFT);
    let plain = mods.difference(Modifiers::SHIFT).is_empty();
    let edge = cfg!(target_os = "macos") && mods.difference(Modifiers::SHIFT) == Modifiers::META;
    let word = mods == Modifiers::ALT;
    match key {
        NamedKey::ArrowLeft | NamedKey::ArrowRight if edge => {
            rt.move_edge(key == NamedKey::ArrowRight, shift);
        }
        NamedKey::ArrowLeft if word => rt.state.move_word_left(),
        NamedKey::ArrowRight if word => rt.state.move_word_right(),
        NamedKey::ArrowLeft | NamedKey::ArrowRight if plain => {
            rt.state.move_by(key == NamedKey::ArrowRight, shift);
        }
        NamedKey::Home | NamedKey::End if plain => rt.move_edge(key == NamedKey::End, shift),
        NamedKey::Backspace if word => rt.edit(TextState::delete_word_backward),
        NamedKey::Delete if word => rt.edit(TextState::delete_word_forward),
        NamedKey::Backspace if plain => rt.edit(TextState::backspace),
        NamedKey::Delete if plain => rt.edit(TextState::delete_forward),
        NamedKey::Enter if mods.is_empty() => (rt.submit)(rt.state.value().peek().as_str()),
        _ => return false,
    }
    rt.keep_caret_visible();
    true
}

fn handle_char_key(event: &KeyEvent, ch: &str, rt: &FieldRuntime) -> bool {
    let mods = event.modifiers();
    let cmd = mods.contains(Modifiers::CTRL)
        || (cfg!(target_os = "macos") && mods.contains(Modifiers::META));
    if !cmd && mods.contains(Modifiers::ALT) {
        match ch {
            "b" => {
                rt.state.move_word_left();
                rt.keep_caret_visible();
                return true;
            }
            "f" => {
                rt.state.move_word_right();
                rt.keep_caret_visible();
                return true;
            }
            "d" => {
                rt.edit(TextState::delete_word_forward);
                return true;
            }
            _ => {}
        }
    }
    if cmd {
        return handle_cmd_char(ch, mods.contains(Modifiers::SHIFT), rt);
    }
    if !mods.intersects(Modifiers::CTRL | Modifiers::META)
        && let Some(typed) = event.text()
        && !typed.is_empty()
        && typed.chars().all(|c| !c.is_control())
    {
        rt.edit(|state| state.insert(typed));
        return true;
    }
    false
}

fn handle_cmd_char(ch: &str, shift: bool, rt: &FieldRuntime) -> bool {
    match ch {
        "a" => {
            rt.state.select_all();
            rt.keep_caret_visible();
        }
        "c" => {
            rt.copy_selection();
        }
        "x" => {
            if rt.copy_selection() {
                rt.edit(TextState::backspace);
            }
        }
        "v" => {
            if let Some(t) = rt.clipboard.get() {
                rt.insert_single_line(&t);
            }
        }
        "w" => rt.edit(TextState::delete_word_backward),
        "u" => rt.edit(|state| {
            state.extend(0);
            state.backspace();
        }),
        "k" => rt.edit(|state| {
            state.extend(state.value().peek().len());
            state.delete_forward();
        }),
        "z" if !shift => rt.edit(|state| {
            let _ = state.undo();
        }),
        "z" | "Z" | "y" => rt.edit(|state| {
            let _ = state.redo();
        }),
        _ => return false,
    }
    true
}

fn attach_field_events(node: NodeBuilder, rt: &FieldRuntime) -> NodeBuilder {
    let (paste_rt, key_rt, anchor_rt) = (rt.clone(), rt.clone(), rt.clone());
    let (disabled, ime_rt, blur_rt, a11y_rt) = (rt.disabled, rt.clone(), rt.clone(), rt.clone());
    let caret_anchor = memo(move || anchor_rt.caret_geometry());
    node.paste(move |text| {
        if !paste_rt.is_disabled() {
            paste_rt.insert_single_line(text);
        }
        true
    })
    .key_any(move |e: &mut KeyEvent| {
        if e.phase() != KeyPhase::Down || key_rt.is_disabled() {
            return;
        }
        let handled = match e.key() {
            Key::Named(named) => handle_named_key(*named, e.modifiers(), &key_rt),
            Key::Character(ch) => handle_char_key(e, ch, &key_rt),
            _ => false,
        };
        if handled {
            e.stop_propagation();
        }
    })
    .ime_eligible(move || !disabled.is_some_and(|flag| flag.peek()))
    .ime_caret(caret_anchor)
    .ime(move |event| ime_rt.apply_ime(event))
    .focus_out(move || blur_rt.clear_preedit())
    .a11y_edit(move |edit| a11y_rt.apply_a11y(edit))
}
