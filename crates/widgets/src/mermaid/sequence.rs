use super::canvas::Canvas;

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
    let mut diagram = Diagram {
        participants: Vec::new(),
        events: Vec::new(),
        numbered: false,
        boxes: 0,
    };
    for line in lines {
        diagram.line(line);
    }
    if diagram.participants.is_empty() {
        return None;
    }
    Some(render(&diagram))
}

impl Diagram {
    fn line(&mut self, line: &str) {
        let head = line.split_whitespace().next().unwrap_or("");
        match head {
            "autonumber" => self.numbered = true,
            "participant" | "actor" => self.declare(line),
            "Note" | "note" => self.note(line),
            "loop" | "alt" | "opt" | "par" | "critical" | "break" | "rect" => {
                self.events.push(Event::Open {
                    kind: head.to_string(),
                    text: line
                        .get(head.len()..)
                        .unwrap_or_default()
                        .trim()
                        .to_string(),
                });
            }
            "else" | "and" | "option" => self.events.push(Event::Else {
                text: line
                    .get(head.len()..)
                    .unwrap_or_default()
                    .trim()
                    .to_string(),
            }),
            "box" => self.boxes = self.boxes.saturating_add(1),
            "end" if self.boxes > 0 => self.boxes = self.boxes.saturating_sub(1),
            "end" => self.events.push(Event::Close),
            "activate" | "deactivate" | "title" | "links" | "link" | "properties" | "details" => {}
            _ => self.message(line),
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
        let (from, to) = if let Some(names) = place.trim().strip_prefix("over ") {
            let mut ids = names.split(',').map(str::trim);
            let first = ids.next().unwrap_or_default();
            let second = ids.next().unwrap_or(first);
            (
                self.participant(first, None),
                self.participant(second, None),
            )
        } else if let Some(name) = place.trim().strip_prefix("left of ") {
            let index = self.participant(name.trim(), None);
            (index.saturating_sub(usize::from(index > 0)), index)
        } else if let Some(name) = place.trim().strip_prefix("right of ") {
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
        let need = halves.saturating_add(3);
        if *gap < need {
            gap.clone_from(&need);
        }
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
        let grown = last.saturating_add(need.saturating_sub(have));
        last.clone_from(&grown);
    }
}

fn render(diagram: &Diagram) -> Vec<String> {
    let depth = depth(&diagram.events);
    let margin = depth.saturating_mul(2).saturating_add(1);
    let (centres, right) = centres(diagram, margin);
    let mut rows = 3_usize;
    for event in &diagram.events {
        let same = matches!(event, Event::Message { from, to, .. } if from == to);
        rows = rows.saturating_add(height(event, same));
    }
    rows = rows.saturating_add(4);
    let cols = right.saturating_add(margin);
    let mut canvas = Canvas::new(cols, rows);
    for (participant, centre) in diagram.participants.iter().zip(&centres) {
        boxed(&mut canvas, participant, *centre, 0);
        boxed(&mut canvas, participant, *centre, rows.saturating_sub(3));
    }
    for centre in &centres {
        canvas.vline(*centre, 3, rows.saturating_sub(4), '┆');
    }
    let frames = events(&mut canvas, diagram, &centres);
    for frame in frames {
        framed(&mut canvas, &frame, cols);
    }
    canvas.lines()
}

fn boxed(canvas: &mut Canvas, participant: &Participant, centre: usize, y: usize) {
    let width = participant.label.chars().count().saturating_add(4);
    let x = centre.saturating_sub(width / 2);
    canvas.frame(x, y, (width, 3), false);
    canvas.text(x.saturating_add(2), y.saturating_add(1), &participant.label);
}

fn events(canvas: &mut Canvas, diagram: &Diagram, centres: &[usize]) -> Vec<Frame> {
    let mut y = 3_usize;
    let mut open: Vec<Frame> = Vec::new();
    let mut done = Vec::new();
    let mut number = 0_usize;
    for event in &diagram.events {
        y = match event {
            Event::Message {
                from,
                to,
                text,
                dashed,
                head,
            } => {
                number = number.saturating_add(1);
                let text = if diagram.numbered {
                    format!("{number}. {text}")
                } else {
                    text.clone()
                };
                let ends = (
                    centres.get(*from).copied().unwrap_or(0),
                    centres.get(*to).copied().unwrap_or(0),
                );
                message(canvas, ends, &text, (*dashed, *head), y)
            }
            Event::Note { from, to, text } => note(canvas, centres, (*from, *to), text, y),
            other => frame_event(&mut open, &mut done, other, y),
        };
    }
    for mut frame in open {
        frame.bottom = y;
        done.push(frame);
    }
    done
}

fn frame_event(open: &mut Vec<Frame>, done: &mut Vec<Frame>, event: &Event, y: usize) -> usize {
    match event {
        Event::Open { kind, text } => open.push(Frame {
            top: y,
            bottom: y,
            depth: open.len(),
            label: format!("{kind} {text}").trim().to_string(),
            elses: Vec::new(),
        }),
        Event::Else { text } => {
            if let Some(frame) = open.last_mut() {
                frame.elses.push((y, text.clone()));
            }
        }
        Event::Close => {
            if let Some(mut frame) = open.pop() {
                frame.bottom = y;
                done.push(frame);
            }
        }
        Event::Message { .. } | Event::Note { .. } => {}
    }
    y.saturating_add(1)
}

fn message(
    canvas: &mut Canvas,
    ends: (usize, usize),
    text: &str,
    style: (bool, char),
    y: usize,
) -> usize {
    let (a, b) = ends;
    let (dashed, head) = style;
    let line = if dashed { '┄' } else { '─' };
    if a == b {
        canvas.text(a.saturating_add(3), y, text);
        canvas.text(
            a.saturating_add(1),
            y.saturating_add(1),
            &format!("{line}{line}┐"),
        );
        canvas.text(
            a.saturating_add(1),
            y.saturating_add(2),
            &format!("◀{line}┘"),
        );
        return y.saturating_add(3);
    }
    let (low, high) = (a.min(b), a.max(b));
    let width = text.chars().count();
    let start = low
        .saturating_add(high)
        .saturating_add(1)
        .checked_div(2)
        .unwrap_or(low)
        .saturating_sub(width / 2)
        .max(low.saturating_add(1));
    canvas.text(start, y, text);
    for x in low.saturating_add(1)..high {
        canvas.set(x, y.saturating_add(1), line);
    }
    if a < b {
        canvas.set(high.saturating_sub(1), y.saturating_add(1), head);
    } else {
        let mirrored = match head {
            '▶' => '◀',
            '>' => '<',
            other => other,
        };
        canvas.set(low.saturating_add(1), y.saturating_add(1), mirrored);
    }
    y.saturating_add(2)
}

fn note(
    canvas: &mut Canvas,
    centres: &[usize],
    span: (usize, usize),
    text: &str,
    y: usize,
) -> usize {
    let (from, to) = span;
    let width = text.chars().count().saturating_add(4);
    let a = centres.get(from.min(to)).copied().unwrap_or(0);
    let b = centres.get(from.max(to)).copied().unwrap_or(a);
    let x = if from == to {
        a.saturating_add(2)
    } else {
        a.saturating_add(b).saturating_add(1) / 2
    }
    .saturating_sub(if from == to { 0 } else { width / 2 });
    canvas.frame(x, y, (width, 3), false);
    canvas.text(x.saturating_add(2), y.saturating_add(1), text);
    y.saturating_add(3)
}

fn framed(canvas: &mut Canvas, frame: &Frame, cols: usize) {
    let left = frame.depth;
    let right = cols.saturating_sub(1).saturating_sub(frame.depth);
    if right <= left.saturating_add(2) {
        return;
    }
    canvas.hline(left, right, frame.top, '─');
    canvas.hline(left, right, frame.bottom, '─');
    canvas.vline(left, frame.top, frame.bottom, '│');
    canvas.vline(right, frame.top, frame.bottom, '│');
    canvas.set(left, frame.top, '┌');
    canvas.set(right, frame.top, '┐');
    canvas.set(left, frame.bottom, '└');
    canvas.set(right, frame.bottom, '┘');
    canvas.text(
        left.saturating_add(2),
        frame.top,
        &format!(" {} ", frame.label),
    );
    for (row, text) in &frame.elses {
        for x in left.saturating_add(1)..right {
            canvas.set(x, *row, '┄');
        }
        canvas.set(left, *row, '├');
        canvas.set(right, *row, '┤');
        canvas.text(left.saturating_add(2), *row, &format!(" {text} "));
    }
}
