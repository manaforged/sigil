use sigil::prelude::*;

#[derive(Clone, Debug)]
pub(crate) struct Message {
    pub(crate) author: String,
    pub(crate) body: String,
}

#[derive(Clone, Debug)]
pub(crate) struct Channel {
    pub(crate) name: String,
    pub(crate) members: Vec<String>,
    pub(crate) pinned: Vec<String>,
    pub(crate) messages: Vec<Message>,
}

pub(crate) const TAB_MESSAGES: usize = 0;
pub(crate) const TAB_MEMBERS: usize = 1;
pub(crate) const TAB_PINNED: usize = 2;

pub(crate) const fn tab_title(index: usize) -> &'static str {
    match index {
        TAB_MESSAGES => "Messages",
        TAB_MEMBERS => "Members",
        TAB_PINNED => "Pinned",
        _ => "?",
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ChatState {
    pub(crate) channels: Signal<Vec<Channel>>,
    pub(crate) selected_channel: Signal<usize>,
    pub(crate) active_tab: Signal<usize>,

    pub(crate) compose_draft: Signal<String>,

    pub(crate) modal_open: Signal<bool>,
    pub(crate) modal_name_draft: Signal<String>,
    pub(crate) modal_topic_draft: Signal<String>,

    pub(crate) split_sizes: Signal<Vec<f32>>,

    pub(crate) status: Signal<String>,
}

impl ChatState {
    pub(crate) fn seed() -> Self {
        let channels = signal(seed_channels());
        Self {
            channels,
            selected_channel: signal(0_usize),
            active_tab: signal(TAB_MESSAGES),
            compose_draft: signal(String::new()),
            modal_open: signal(false),
            modal_name_draft: signal(String::new()),
            modal_topic_draft: signal(String::new()),
            split_sizes: signal(vec![240.0, 760.0]),
            status: signal(String::from("Ready. Pick a channel and start typing.")),
        }
    }

    pub(crate) fn open_channel_settings(self) {
        let idx = self.selected_channel.peek();
        let name = self
            .channels
            .peek()
            .get(idx)
            .map(|c| c.name.clone())
            .unwrap_or_default();
        self.modal_name_draft.set(name);
        let topic = self
            .channels
            .peek()
            .get(idx)
            .and_then(|c| c.pinned.first().cloned())
            .unwrap_or_default();
        self.modal_topic_draft.set(topic);
        self.modal_open.set(true);
        self.status
            .set(String::from("Editing channel settings — Esc to cancel."));
    }

    pub(crate) fn commit_modal(self) {
        let name = self.modal_name_draft.peek();
        let topic = self.modal_topic_draft.peek();
        if name.trim().is_empty() {
            self.status
                .set(String::from("Channel name is empty — modal stays open."));
            return;
        }
        let idx = self.selected_channel.peek();
        self.channels.update(|chans| {
            if let Some(chan) = chans.get_mut(idx) {
                chan.name = name.clone();
                if topic.trim().is_empty() {
                    chan.pinned.clear();
                } else if let Some(slot) = chan.pinned.first_mut() {
                    *slot = topic.clone();
                } else {
                    chan.pinned.push(topic.clone());
                }
            }
        });
        self.modal_open.set(false);
        self.status.set(format!("Saved channel: {name}"));
    }

    pub(crate) fn cancel_modal(self) {
        self.modal_open.set(false);
        self.status.set(String::from("Channel edit cancelled."));
    }

    pub(crate) fn select_channel(self, idx: usize) {
        self.selected_channel.set(idx);
        let name = self
            .channels
            .peek()
            .get(idx)
            .map_or_else(|| String::from("?"), |c| c.name.clone());
        self.status.set(format!("Channel: {name}"));
    }

    pub(crate) fn send_compose(self) {
        let body = self.compose_draft.peek();
        if body.trim().is_empty() {
            self.status
                .set(String::from("Nothing to send — compose draft is empty."));
            return;
        }
        let idx = self.selected_channel.peek();
        self.channels.update(|chans| {
            if let Some(chan) = chans.get_mut(idx) {
                chan.messages.push(Message {
                    author: String::from("you"),
                    body: body.clone(),
                });
            }
        });
        self.compose_draft.set(String::new());
        self.status.set(String::from("Sent."));
    }
}

fn seed_channels() -> Vec<Channel> {
    let general_messages: Vec<Message> = (0..40)
        .map(|i| Message {
            author: if i % 3 == 0 {
                String::from("ada")
            } else if i % 3 == 1 {
                String::from("grace")
            } else {
                String::from("linus")
            },
            body: format!("Message #{i} in general. Sigil dogfood feedback follows."),
        })
        .collect();
    let random_messages: Vec<Message> = (0..18)
        .map(|i| Message {
            author: String::from("anon"),
            body: format!("Random thought #{i}."),
        })
        .collect();
    vec![
        Channel {
            name: String::from("#general"),
            members: (0..32).map(|i| format!("member-{i}")).collect(),
            pinned: vec![String::from("Read the README before posting.")],
            messages: general_messages,
        },
        Channel {
            name: String::from("#random"),
            members: vec![String::from("ada"), String::from("grace")],
            pinned: Vec::new(),
            messages: random_messages,
        },
        Channel {
            name: String::from("#dogfood"),
            members: (0..12).map(|i| format!("tester-{i}")).collect(),
            pinned: vec![
                String::from("Dogfood v2 in flight."),
                String::from("Friction log: examples/chat-client/FRICTION.md"),
            ],
            messages: (0..8)
                .map(|i| Message {
                    author: String::from("tester"),
                    body: format!("Bug report #{i}: text_edit reactive children freeze."),
                })
                .collect(),
        },
    ]
}
