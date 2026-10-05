#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;
use sigil::tasks::sleep;
use sigil::widgets::{ModalBuilder, ScrollBuilder, TextFieldBuilder};
use std::time::Duration;
use tokio::sync::mpsc::{
    UnboundedReceiver as Receiver, UnboundedSender as Sender, unbounded_channel,
};

const PANE_PADDING: f32 = sigil::style::space(2);
const CHANNELS: &str = "general random builds incidents design release";

const LINES: &[&str] = &[
    "general ci: build 4812 finished on linux-x64 in 3m12s, all 1,204 tests green",
    "general ana: has anyone looked at the flaky snapshot test in the renderer crate?",
    "general mo: I can take it after lunch, it only fails on the dark theme run",
    "general ci: release candidate rc.3 is tagged and the notes are drafted",
    "general lee: lunch order closes in ten minutes, reply here with your pick",
    "incidents pager: latency p99 on the gateway crossed 800 ms for five minutes",
    "design mo: pushed the new sidebar spacing, screenshots in the design doc",
];

#[derive(Clone, Debug, PartialEq)]
struct Msg {
    channel: usize,
    author: String,
    body: String,
    done: bool,
}

fn seed(line: &&str) -> Msg {
    let (channel, line) = line.split_once(' ').unwrap_or_default();
    let (author, body) = line.split_once(": ").unwrap_or_default();
    let mut names = CHANNELS.split(' ');
    Msg {
        channel: names.position(|name| name == channel).unwrap_or_default(),
        author: author.to_owned(),
        body: body.to_owned(),
        done: true,
    }
}

struct Post {
    channel: usize,
    body: String,
}

#[derive(Clone, Copy)]
struct Chat {
    messages: Signal<Vec<Msg>>,
    online: Signal<&'static str>,
    server: Signal<Status<(), String>>,
    names: Signal<Vec<String>>,
    active: Signal<usize>,
    unread: Signal<Vec<usize>>,
    visible: Signal<Vec<usize>>,
}

#[derive(Clone, Copy)]
struct Picker {
    open: Signal<bool>,
    query: Signal<String>,
    cursor: Signal<usize>,
    focus: Signal<u64>,
}

struct Server {
    messages: SignalWriter<Vec<Msg>>,
    online: SignalWriter<&'static str>,
    posts: Receiver<Post>,
}

impl Server {
    async fn run(mut self) -> Result<(), ScheduleError> {
        sleep(Duration::from_millis(800)).await;
        self.online.set("● online")?;
        while let Some(post) = self.posts.recv().await {
            self.echo(&post).await?;
        }
        Ok(())
    }

    async fn echo(&self, post: &Post) -> Result<(), ScheduleError> {
        self.stream(post.channel, "you", &post.body).await?;
        let words = post.body.split_whitespace().count();
        let reply = format!(
            "I read {words} words. Streaming a longer reply so the transcript wraps and follows the bottom edge while tokens arrive."
        );
        self.stream(post.channel, "bot", &reply).await
    }

    async fn stream(&self, channel: usize, author: &str, body: &str) -> Result<(), ScheduleError> {
        let author = author.to_owned();
        self.messages.update(move |all| {
            all.push(Msg {
                channel,
                author,
                body: String::new(),
                done: false,
            });
        })?;
        for word in body.split_inclusive(' ') {
            let word = word.to_owned();
            self.messages.update(move |all| {
                if let Some(msg) = all.last_mut() {
                    msg.body.push_str(&word);
                }
            })?;
            sleep(Duration::from_millis(45)).await;
        }
        self.messages
            .update(|all| all.last_mut().into_iter().for_each(|msg| msg.done = true))
    }
}

fn main() -> sigil::AppResult<()> {
    let config = AppConfig::new("sigil-chat");
    let config = if std::env::args().any(|arg| arg == "--gui") {
        config.size(1040, 660).window()
    } else {
        config.terminal()
    };
    sigil::run(config, app_tree)
}

fn start() -> (Chat, Sender<Post>) {
    let messages = signal(LINES.iter().map(seed).collect::<Vec<_>>());
    let online = signal("◌ connecting");
    let (posts, rx) = unbounded_channel();
    let backend = Server {
        messages: signal_writer(messages),
        online: signal_writer(online),
        posts: rx,
    };
    let server = spawn(async move {
        match backend.run().await {
            Ok(()) | Err(ScheduleError::Closed) => Ok(()),
            Err(err) => Err(err.to_string()),
        }
    });
    (model(messages, online, server), posts)
}

fn model(
    messages: Signal<Vec<Msg>>,
    online: Signal<&'static str>,
    server: Signal<Status<(), String>>,
) -> Chat {
    let names = signal(CHANNELS.split(' ').map(str::to_owned).collect::<Vec<_>>());
    let active = signal(0_usize);
    let seen = signal(vec![0_usize; 6]);
    let unread = derived(move || {
        let marks = seen.get();
        messages.with(|all| {
            let count = |(ch, skip): (usize, &usize)| {
                all.iter()
                    .skip(*skip)
                    .filter(|msg| msg.channel == ch)
                    .count()
            };
            marks.iter().enumerate().map(count).collect::<Vec<_>>()
        })
    });
    let visible = derived(move || {
        let current = active.get();
        messages.with(|all| {
            let hit = |(index, msg): (usize, &Msg)| (msg.channel == current).then_some(index);
            all.iter().enumerate().filter_map(hit).collect::<Vec<_>>()
        })
    });
    effect(move || {
        let current = active.get();
        let total = messages.with(Vec::len);
        seen.update(|marks| {
            if let Some(mark) = marks.get_mut(current) {
                *mark = total;
            }
        });
    });
    Chat {
        messages,
        online,
        server,
        names,
        active,
        unread,
        visible,
    }
}

fn link_text(chat: Chat) -> String {
    if let Status::Err(err) = chat.server.get() {
        return format!("backend error: {err}");
    }
    chat.online.get().to_owned()
}

fn current_name(chat: Chat) -> String {
    let index = chat.active.get();
    chat.names
        .with(|names| names.get(index).cloned().unwrap_or_default())
}

pub(crate) fn app_tree() -> impl IntoNode {
    let (chat, posts) = start();
    let picker = Picker {
        open: signal(false),
        query: signal(String::new()),
        cursor: signal(0),
        focus: signal(0),
    };
    let open_picker = move |_: &mut KeyEvent| {
        picker.query.set(String::new());
        picker.cursor.set(0);
        picker.open.set(true);
        picker.focus.update(|n| *n += 1);
    };
    let unread = move || format!("{} unread", chat.unread.with(|u| u.iter().sum::<usize>()));
    container()
        .layout(LayoutStyle::flex_column().fill())
        .key_capture(Shortcut::ctrl(LogicalKey::character("k")), open_picker)
        .child(
            container()
                .layout(LayoutStyle::flex_row().fill_width().padding(PANE_PADDING))
                .child(text(move || format!("# {}", current_name(chat))).nowrap())
                .child(spacer())
                .child(text("sigil-chat").nowrap().font_size(14.0)),
        )
        .child(work(chat, posts))
        .child(status_bar([
            text(move || link_text(chat)).nowrap(),
            text(unread).nowrap(),
            text("^K switch channel  Enter send").nowrap(),
        ]))
        .child(picker_modal(chat, picker))
}

fn work(chat: Chat, posts: Sender<Post>) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .grow(1.0)
                .min_height(rows(0.0)),
        )
        .child(sidebar(channels(chat)))
        .child(
            container()
                .layout(
                    LayoutStyle::flex_column()
                        .fill()
                        .gap(rows(1.0))
                        .padding(PANE_PADDING),
                )
                .child(transcript(chat))
                .child(composer(chat, posts)),
        )
}

fn channels(chat: Chat) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().fill_width())
        .child(section_label("Channels"))
        .children_reactive(move || {
            let current = chat.active.get();
            let entries = chat
                .names
                .get()
                .into_iter()
                .zip(chat.unread.get())
                .enumerate();
            entries
                .map(|(index, (name, unread))| {
                    let slots = ListRowSlots {
                        status: (unread > 0).then(|| TextContent::from(unread.to_string())),
                        ..ListRowSlots::default()
                    };
                    list_row_with(index == current, format!("# {name}"), slots)
                        .pointer_up(move |_| chat.active.set(index))
                })
                .collect::<Vec<_>>()
        })
}

fn transcript(chat: Chat) -> ScrollBuilder {
    scroll(
        container()
            .layout(LayoutStyle::flex_column().fill_width().gap(rows(1.0)))
            .children_reactive(move || {
                let entries = chat.visible.get().into_iter();
                entries
                    .map(|index| message(chat, index))
                    .collect::<Vec<_>>()
            }),
    )
    .fill()
    .follow_end(true)
}

fn message(chat: Chat, index: usize) -> NodeBuilder {
    let read = move |f: fn(&Msg) -> String| chat.messages.with(|all| all.get(index).map(f));
    let head = move || {
        let head = |msg: &Msg| format!("{}{}", msg.author, if msg.done { "" } else { " · typing" });
        read(head).unwrap_or_default()
    };
    let body = text(move || read(|msg| msg.body.clone()).unwrap_or_default());
    container()
        .layout(LayoutStyle::flex_column().fill_width())
        .child(
            text(head)
                .nowrap()
                .font_size(14.0)
                .text_color(Token::Accent),
        )
        .child(body.wrap())
}

fn composer(chat: Chat, posts: Sender<Post>) -> TextFieldBuilder {
    let draft = signal(String::new());
    text_field(draft)
        .autofocus()
        .placeholder(move || format!("Message # {}", current_name(chat)))
        .submit(move |value| {
            let post = Post {
                channel: chat.active.peek(),
                body: value.to_owned(),
            };
            if !value.trim().is_empty() && posts.send(post).is_ok() {
                draft.set(String::new());
            }
        })
}

fn rank(names: &[String], query: &str) -> Vec<usize> {
    if query.trim().is_empty() {
        return (0..names.len()).collect();
    }
    let mut hits: Vec<(usize, usize)> = names
        .iter()
        .enumerate()
        .filter_map(|(index, name)| fuzzy_score(query, name).map(|hit| (hit.score, index)))
        .collect();
    hits.sort_unstable();
    hits.into_iter().map(|(_, index)| index).collect()
}

fn pick(chat: Chat, picker: Picker, channel: usize) {
    chat.active.set(channel);
    picker.open.set(false);
}

fn picker_modal(chat: Chat, picker: Picker) -> ModalBuilder {
    let hits = derived(move || {
        let query = picker.query.get();
        chat.names.with(|names| rank(names, &query))
    });
    let step = move |forward: bool| {
        let count = hits.with(Vec::len).max(1);
        let shift = if forward { 1 } else { count - 1 };
        picker.cursor.update(|at| *at = (*at + shift) % count);
    };
    let search = text_field(picker.query)
        .placeholder("Jump to channel")
        .focus_when(picker.focus)
        .change(move |_| picker.cursor.set(0))
        .submit(move |_| {
            if let Some(channel) = hits.with(|h| h.get(picker.cursor.peek()).copied()) {
                pick(chat, picker, channel);
            }
        });
    let results = container()
        .layout(LayoutStyle::flex_column().fill_width())
        .children_reactive(move || {
            let cursor = picker.cursor.get();
            let names = chat.names.get();
            let entries = hits.get().into_iter().enumerate();
            entries
                .filter_map(|(row, channel)| {
                    let name = names.get(channel)?;
                    let item = list_row(row == cursor, format!("# {name}"), "");
                    Some(item.pointer_up(move |_| pick(chat, picker, channel)))
                })
                .collect::<Vec<_>>()
        });
    modal(picker.open)
        .title("Switch channel")
        .dismiss_on_backdrop(true)
        .child(
            container()
                .layout(LayoutStyle::flex_column().fill_width().gap(rows(0.5)))
                .key_capture(Key::ArrowDown, move |_| step(true))
                .key_capture(Key::ArrowUp, move |_| step(false))
                .child(search)
                .child(results),
        )
}
