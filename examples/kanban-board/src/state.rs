use sigil::prelude::*;

#[derive(Clone, Debug)]
pub(crate) struct Card {
    pub(crate) title: String,
    pub(crate) description: String,
}

pub(crate) const COL_TODO: usize = 0;
pub(crate) const COL_DOING: usize = 1;
pub(crate) const COL_DONE: usize = 2;

#[derive(Clone, Copy)]
pub(crate) struct BoardState {
    pub(crate) projects: Signal<Vec<String>>,
    pub(crate) selected_project: Signal<usize>,
    pub(crate) columns: Signal<[Vec<Card>; 3]>,
    pub(crate) active_tab: Signal<usize>,

    pub(crate) modal_open: Signal<bool>,
    pub(crate) edit_title: Signal<String>,
    pub(crate) edit_description: Signal<String>,
    pub(crate) edit_slot: Signal<Option<(usize, usize)>>,

    pub(crate) split_sizes: Signal<Vec<f32>>,

    pub(crate) status: Signal<String>,
}

impl BoardState {
    pub(crate) fn seed() -> Self {
        let projects = signal(vec![
            String::from("Sigil dogfood"),
            String::from("Marketing site"),
            String::from("Engine port"),
        ]);

        let initial_cards: [Vec<Card>; 3] = [
            (0..40)
                .map(|i| Card {
                    title: format!("Todo task #{i}"),
                    description: String::from("Pending triage."),
                })
                .collect(),
            (0..12)
                .map(|i| Card {
                    title: format!("Doing task #{i}"),
                    description: String::from("In flight."),
                })
                .collect(),
            (0..8)
                .map(|i| Card {
                    title: format!("Done task #{i}"),
                    description: String::from("Shipped."),
                })
                .collect(),
        ];

        Self {
            projects,
            selected_project: signal(0_usize),
            columns: signal(initial_cards),
            active_tab: signal(COL_TODO),
            modal_open: signal(false),
            edit_title: signal(String::new()),
            edit_description: signal(String::new()),
            edit_slot: signal(None),
            split_sizes: signal(vec![220.0, 720.0]),
            status: signal(String::from("Ready. Pick a column tab and click a card.")),
        }
    }

    pub(crate) fn open_new_card(self) {
        self.edit_title.set(String::new());
        self.edit_description.set(String::new());
        self.edit_slot.set(None);
        self.modal_open.set(true);
        self.status
            .set(String::from("New card — type a title, press Enter."));
    }

    pub(crate) fn open_edit(self, col: usize, idx: usize) {
        let cards = self.columns.peek();
        let Some(column) = cards.get(col) else { return };
        let Some(card) = column.get(idx) else { return };
        self.edit_title.set(card.title.clone());
        self.edit_description.set(card.description.clone());
        self.edit_slot.set(Some((col, idx)));
        self.modal_open.set(true);
        self.status.set(format!("Editing {}", card.title));
    }

    pub(crate) fn commit_modal(self) {
        let title = self.edit_title.peek();
        let description = self.edit_description.peek();
        if title.trim().is_empty() {
            self.status
                .set(String::from("Title is empty — modal stays open."));
            return;
        }
        let slot = self.edit_slot.peek();
        let active = self.active_tab.peek();
        self.columns.update(|cols| match slot {
            Some((col, idx)) => {
                if let Some(column) = cols.get_mut(col)
                    && let Some(card) = column.get_mut(idx)
                {
                    card.title = title.clone();
                    card.description = description.clone();
                }
            }
            None => {
                if let Some(column) = cols.get_mut(active) {
                    column.push(Card {
                        title: title.clone(),
                        description: description.clone(),
                    });
                }
            }
        });
        self.modal_open.set(false);
        self.status.set(format!("Saved card: {title}"));
    }

    pub(crate) fn cancel_modal(self) {
        self.modal_open.set(false);
        self.status.set(String::from("Edit cancelled."));
    }

    pub(crate) fn move_card(self, from: usize, idx: usize, to: usize) {
        if from == to {
            return;
        }
        self.columns.update(|cols| {
            let card = cols
                .get_mut(from)
                .and_then(|column| (idx < column.len()).then(|| column.remove(idx)));
            if let Some(card) = card
                && let Some(dest) = cols.get_mut(to)
            {
                dest.push(card);
            }
        });
        self.status
            .set(format!("Moved card to {}", column_title(to)));
    }

    pub(crate) fn select_project(self, idx: usize) {
        self.selected_project.set(idx);
        let name = self
            .projects
            .peek()
            .get(idx)
            .cloned()
            .unwrap_or_else(|| String::from("?"));
        self.status.set(format!("Project: {name}"));
    }
}

#[allow(
    unreachable_pub,
    clippy::redundant_pub_crate,
    reason = "demo harness; deliberate"
)]
pub(crate) const fn column_title(index: usize) -> &'static str {
    match index {
        COL_TODO => "Todo",
        COL_DOING => "Doing",
        COL_DONE => "Done",
        _ => "?",
    }
}
