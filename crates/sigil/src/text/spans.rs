use crate::style::{TextFace, TextRun};

/// Text built from styled spans that shape, wrap, and select as one paragraph.
#[must_use]
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Spans {
    text: String,
    runs: Vec<TextRun>,
}

impl Spans {
    /// Creates spans that hold no text.
    pub const fn new() -> Self {
        Self {
            text: String::new(),
            runs: Vec::new(),
        }
    }

    /// Appends `text` drawn with `face` over the paragraph style.
    pub fn span(mut self, text: impl AsRef<str>, face: TextFace) -> Self {
        self.push(text.as_ref(), &face);
        self
    }

    /// The plain text of every span in order, as screen readers and selection see it.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    pub(crate) fn push(&mut self, text: &str, face: &TextFace) {
        let start = self.text.len();
        self.text.push_str(text);
        if !text.is_empty() && *face != TextFace::new() {
            self.runs.push(TextRun {
                start,
                end: self.text.len(),
                face: face.clone(),
            });
        }
    }

    pub(crate) fn into_parts(self) -> (String, Vec<TextRun>) {
        (self.text, self.runs)
    }
}

impl<S: AsRef<str>> FromIterator<(S, TextFace)> for Spans {
    fn from_iter<I: IntoIterator<Item = (S, TextFace)>>(iter: I) -> Self {
        iter.into_iter()
            .fold(Self::new(), |spans, (text, face)| spans.span(text, face))
    }
}
