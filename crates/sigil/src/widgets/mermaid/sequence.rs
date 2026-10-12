mod paint;

use paint::render;

const MIN_GAP: usize = 6;

struct Participant {
    id: String,
    label: String,
}

enum Event {
    Message {
        from: usize,
        to: usize,
        text: String,
        dashed: bool,
        head: char,
    },
    Note {
        from: usize,
        to: usize,
        text: String,
    },
    Open {
        kind: String,
        text: String,
    },
    Else {
        text: String,
    },
    Close,
}

#[derive(Default)]
struct Diagram {
    participants: Vec<Participant>,
    events: Vec<Event>,
    numbered: bool,
    boxes: usize,
}

struct Frame {
    top: usize,
    bottom: usize,
    depth: usize,
    label: String,
    elses: Vec<(usize, String)>,
}

pub(super) fn draw(source: &str) -> Option<Vec<String>> {
    let mut lines = source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("%%"));
    if lines.next()? != "sequenceDiagram" {
        return None;
    }
    let mut diagram = Diagram::default();
    for line in lines {
        diagram.line(line);
    }
    if diagram.participants.is_empty() {
        return None;
    }
    Some(render(&diagram))
}

const IGNORED: &[&str] = &[
    "activate",
    "deactivate",
    "title",
    "links",
    "link",
    "properties",
    "details",
];

#[derive(Clone, Copy)]
enum Keyword {
    Autonumber,
    Declare,
    Note,
    Open,
    Else,
    Box,
    End,
    Ignored,
}

impl Keyword {
    fn of(head: &str) -> Option<Self> {
        Some(match head {
            "autonumber" => Self::Autonumber,
            "participant" | "actor" => Self::Declare,
            "Note" | "note" => Self::Note,
            "loop" | "alt" | "opt" | "par" | "critical" | "break" | "rect" => Self::Open,
            "else" | "and" | "option" => Self::Else,
            "box" => Self::Box,
            "end" => Self::End,
            _ if IGNORED.contains(&head) => Self::Ignored,
            _ => return None,
        })
    }
}

impl Diagram {
    fn line(&mut self, line: &str) {
        let head = line.split_whitespace().next().unwrap_or("");
        let text = || {
            line.get(head.len()..)
                .unwrap_or_default()
                .trim()
                .to_string()
        };
        match Keyword::of(head) {
            Some(Keyword::Autonumber) => self.numbered = true,
            Some(Keyword::Declare) => self.declare(line),
            Some(Keyword::Note) => self.note(line),
            Some(Keyword::Open) => self.events.push(Event::Open {
                kind: head.to_string(),
                text: text(),
            }),
            Some(Keyword::Else) => self.events.push(Event::Else { text: text() }),
            Some(Keyword::Box) => self.boxes = self.boxes.saturating_add(1),
            Some(Keyword::End) if self.boxes > 0 => self.boxes = self.boxes.saturating_sub(1),
            Some(Keyword::End) => self.events.push(Event::Close),
            Some(Keyword::Ignored) => {}
            None => self.message(line),
        }
    }

    fn declare(&mut self, line: &str) {
        let rest = line
            .split_once(char::is_whitespace)
            .map(|(_, rest)| rest.trim())
            .unwrap_or_default();
        let (id, label) = rest
            .split_once(" as ")
            .map_or((rest, rest), |(id, label)| (id.trim(), label.trim()));
        self.participant(id, Some(label));
    }

    fn participant(&mut self, id: &str, label: Option<&str>) -> usize {
        if let Some(index) = self.participants.iter().position(|p| p.id == id) {
            if let (Some(label), Some(found)) = (label, self.participants.get_mut(index)) {
                found.label = label.to_string();
            }
            return index;
        }
        self.participants.push(Participant {
            id: id.to_string(),
            label: label.unwrap_or(id).to_string(),
        });
        self.participants.len().saturating_sub(1)
    }

    fn note(&mut self, line: &str) {
        let rest = line.get(4..).unwrap_or_default().trim();
        let Some((place, text)) = rest.split_once(':') else {
            return;
        };
        let text = text.trim().to_string();
        let place = place.trim();
        let (from, to) = if let Some(names) = place.strip_prefix("over ") {
            let mut ids = names.split(',').map(str::trim);
            let first = ids.next().unwrap_or_default();
            let second = ids.next().unwrap_or(first);
            (
                self.participant(first, None),
                self.participant(second, None),
            )
        } else if let Some(name) = place.strip_prefix("left of ") {
            let index = self.participant(name.trim(), None);
            (index.saturating_sub(usize::from(index > 0)), index)
        } else if let Some(name) = place.strip_prefix("right of ") {
            let index = self.participant(name.trim(), None);
            (index, index.saturating_add(1))
        } else {
            return;
        };
        self.events.push(Event::Note { from, to, text });
    }

    fn message(&mut self, line: &str) {
        const ARROWS: [&str; 10] = [
            "<<-->>", "<<->>", "-->>", "->>", "--)", "-)", "--x", "-x", "-->", "->",
        ];
        let Some((arrow, at)) = ARROWS
            .iter()
            .filter_map(|arrow| line.find(arrow).map(|at| (*arrow, at)))
            .min_by_key(|(arrow, at)| (*at, usize::MAX - arrow.len()))
        else {
            return;
        };
        let from = line.get(..at).unwrap_or_default().trim();
        let rest = line
            .get(at.saturating_add(arrow.len())..)
            .unwrap_or_default();
        let (to, text) = rest
            .split_once(':')
            .map_or_else(|| (rest.trim(), ""), |(to, text)| (to.trim(), text.trim()));
        if from.is_empty() || to.is_empty() {
            return;
        }
        let from = self.participant(from, None);
        let to = self.participant(to, None);
        let head = if arrow.ends_with(">>") {
            '▶'
        } else if arrow.ends_with('x') {
            '✕'
        } else if arrow.ends_with(')') {
            ')'
        } else {
            '>'
        };
        self.events.push(Event::Message {
            from,
            to,
            text: text.to_string(),
            dashed: arrow.starts_with("--") || arrow.starts_with("<<--"),
            head,
        });
    }
}

const fn height(event: &Event, from_to_same: bool) -> usize {
    match event {
        Event::Message { .. } if from_to_same => 3,
        Event::Message { .. } => 2,
        Event::Note { .. } => 3,
        Event::Open { .. } | Event::Else { .. } | Event::Close => 1,
    }
}

fn depth(events: &[Event]) -> usize {
    let mut current = 0_usize;
    let mut deepest = 0_usize;
    for event in events {
        match event {
            Event::Open { .. } => {
                current = current.saturating_add(1);
                deepest = deepest.max(current);
            }
            Event::Close => current = current.saturating_sub(1),
            _ => {}
        }
    }
    deepest
}

fn centres(diagram: &Diagram, margin: usize) -> (Vec<usize>, usize) {
    let widths: Vec<usize> = diagram
        .participants
        .iter()
        .map(|p| p.label.chars().count().saturating_add(4))
        .collect();
    let count = widths.len();
    let mut gaps = vec![MIN_GAP; count.saturating_sub(1)];
    for (index, gap) in gaps.iter_mut().enumerate() {
        let halves = widths.get(index).copied().unwrap_or(0) / 2
            + widths.get(index.saturating_add(1)).copied().unwrap_or(0) / 2;
        *gap = (*gap).max(halves.saturating_add(3));
    }
    for event in &diagram.events {
        widen(&mut gaps, event, diagram.numbered);
    }
    let mut centres = Vec::with_capacity(count);
    let mut x = margin.saturating_add(widths.first().copied().unwrap_or(0) / 2);
    for index in 0..count {
        centres.push(x);
        x = x.saturating_add(gaps.get(index).copied().unwrap_or(0));
    }
    let right = centres
        .last()
        .copied()
        .unwrap_or(0)
        .saturating_add(widths.last().copied().unwrap_or(0) / 2)
        .saturating_add(1);
    (centres, right)
}

fn widen(gaps: &mut [usize], event: &Event, numbered: bool) {
    let extra = if numbered { 4 } else { 0 };
    let (from, to, need) = match event {
        Event::Message { from, to, text, .. } if from == to => (
            *from,
            from.saturating_add(1),
            text.chars().count().saturating_add(6),
        ),
        Event::Message { from, to, text, .. } => {
            (*from, *to, text.chars().count().saturating_add(4 + extra))
        }
        Event::Note { from, to, text } => (*from, *to, text.chars().count().saturating_add(4)),
        _ => return,
    };
    let (low, high) = (from.min(to), from.max(to));
    if low == high {
        return;
    }
    let have: usize = gaps.get(low..high).map_or(0, |slice| slice.iter().sum());
    if have < need
        && let Some(last) = gaps.get_mut(high.saturating_sub(1))
    {
        *last = last.saturating_add(need.saturating_sub(have));
    }
}
