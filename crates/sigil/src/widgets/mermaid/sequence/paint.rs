use super::super::canvas::Canvas;
use super::{Diagram, Event, Frame, Participant, centres, depth, height};

pub(super) fn render(diagram: &Diagram) -> Vec<String> {
    let depth = depth(&diagram.events);
    let margin = depth.saturating_mul(2).saturating_add(1);
    let (centres, right) = centres(diagram, margin);
    let rows = diagram.events.iter().fold(7_usize, |rows, event| {
        let same = matches!(event, Event::Message { from, to, .. } if from == to);
        rows.saturating_add(height(event, same))
    });
    let cols = right.saturating_add(margin);
    let mut canvas = Canvas::new(cols, rows);
    for (participant, centre) in diagram.participants.iter().zip(&centres) {
        boxed(&mut canvas, participant, *centre, 0);
        boxed(&mut canvas, participant, *centre, rows.saturating_sub(3));
    }
    for centre in &centres {
        canvas.vline(*centre, 3, rows.saturating_sub(4), '┆');
    }
    for frame in events(&mut canvas, diagram, &centres) {
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
