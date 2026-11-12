pub const MESSAGES: usize = 200;
pub const STREAM_LIMIT: usize = 4_000;
pub const VIEW: (f32, f32) = (1280.0, 800.0);
pub const NARROW: (f32, f32) = (1272.0, 800.0);
pub const FRAME_SECONDS: f32 = 1.0 / 60.0;
pub const WHEEL_LINE: f32 = 40.0;
pub const WHEEL_ROWS: f32 = WHEEL_LINE / sigil::Host::CELL;
pub const COLS: u16 = 120;
pub const LINES: u16 = 40;
pub const BODY: &str = "Layout resolves each box from its style and its constraints, and text wraps to the width that it gets from its parent.";

/// The window size, one cell narrower when `narrow` is set.
pub const fn view(narrow: bool) -> (f32, f32) {
    if narrow { NARROW } else { VIEW }
}

/// The title of one message.
pub fn title(index: usize) -> String {
    format!("Message {index}")
}

/// The titles of every message.
pub fn titles() -> Vec<String> {
    (0..MESSAGES).map(title).collect()
}

/// Grows the streaming text by one word.
pub fn extend(body: &mut String) {
    if body.len() > STREAM_LIMIT {
        body.clear();
    }
    body.push_str("word ");
}
