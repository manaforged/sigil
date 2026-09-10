use sigil::prelude::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Page {
    Activity,
    Schedules,
    Tools,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Speaker {
    User,
    System,
}

#[derive(Clone, Debug)]
pub(crate) struct Line {
    pub speaker: Speaker,
    pub body: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Where {
    Local,
    Worktree,
    Cloud,
}

#[derive(Clone, Debug)]
pub(crate) struct Thread {
    pub id: u32,
    pub project: &'static str,
    pub title: String,
    pub age: &'static str,
    pub pinned: bool,
    pub running: bool,
    pub lines: Vec<Line>,
}

#[derive(Clone, Copy)]
pub(crate) struct Shell {
    pub page: Signal<Page>,
    pub selected: Signal<u32>,
    pub threads: Signal<Vec<Thread>>,
    pub draft: Signal<String>,
    pub run: Signal<Where>,
    pub show_diff: Signal<bool>,
    pub auto_name: Signal<String>,
    pub auto_prompt: Signal<String>,
    pub auto_days: Signal<u8>,
    pub auto_daily: Signal<bool>,
    pub note: Signal<String>,
}

impl Shell {
    pub(crate) fn seed() -> Self {
        let threads = seed_threads();
        let selected = threads.get(3).map_or(1, |thread| thread.id);
        Self {
            page: signal(Page::Activity),
            selected: signal(selected),
            threads: signal(threads),
            draft: signal(String::new()),
            run: signal(Where::Local),
            show_diff: signal(false),
            auto_name: signal(String::from(
                "Refresh project notes and maintenance checklist",
            )),
            auto_prompt: signal(String::from(
                "Summarize completed work, resolved defects, and remaining maintenance items.",
            )),
            auto_days: signal(0b0011_1110),
            auto_daily: signal(true),
            note: signal(String::from("let's revise this")),
        }
    }

    pub(crate) fn new_thread(&self) {
        self.page.set(Page::Activity);
        self.selected.set(0);
        self.draft.set(String::new());
    }

    pub(crate) fn toggle_day(&self, day: u8) {
        let bit = 1_u8 << day.min(6);
        self.auto_days.update(|mask| *mask ^= bit);
    }

    pub(crate) fn day_on(&self, day: u8) -> bool {
        self.auto_days.get() & (1_u8 << day.min(6)) != 0
    }

    pub(crate) fn toggle_diff(&self) {
        self.show_diff.update(|on| *on = !*on);
    }

    pub(crate) fn current(&self) -> Option<Thread> {
        let id = self.selected.get();
        self.threads.get().into_iter().find(|t| t.id == id)
    }

    pub(crate) fn select(&self, id: u32) {
        self.page.set(Page::Activity);
        self.selected.set(id);
    }

    pub(crate) fn send(&self) {
        let text = self.draft.get();
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return;
        }
        let id = self.selected.peek();
        if id == 0 {
            let next = self
                .threads
                .peek()
                .iter()
                .map(|thread| thread.id)
                .max()
                .unwrap_or(0)
                + 1;
            self.threads.update(|rows| {
                rows.insert(
                    0,
                    Thread {
                        id: next,
                        project: "local-workbench",
                        title: clip(trimmed),
                        age: "now",
                        pinned: false,
                        running: false,
                        lines: vec![user(trimmed)],
                    },
                );
            });
            self.selected.set(next);
        } else {
            let line = user(trimmed);
            self.threads.update(|rows| {
                if let Some(thread) = rows.iter_mut().find(|t| t.id == id) {
                    thread.lines.push(line);
                }
            });
        }
        self.draft.set(String::new());
    }
}

fn clip(body: &str) -> String {
    let mut title: String = body.chars().take(36).collect();
    if body.chars().count() > 36 {
        title.push('…');
    }
    title
}

fn seed_threads() -> Vec<Thread> {
    vec![
        Thread {
            id: 1,
            project: "local-workbench",
            title: "Prepare the morning build".into(),
            age: "2h",
            pinned: false,
            running: false,
            lines: vec![system("Build queue is ready.")],
        },
        Thread {
            id: 2,
            project: "local-workbench",
            title: "List workspace contents".into(),
            age: "3h",
            pinned: false,
            running: true,
            lines: vec![system("Listed the workspace contents.")],
        },
        Thread {
            id: 3,
            project: "shot-scraper",
            title: "Validate tests on dependency...".into(),
            age: "3h",
            pinned: true,
            running: false,
            lines: vec![system("Tests are green on the pinned dependencies.")],
        },
        Thread {
            id: 4,
            project: "shot-scraper",
            title: "Document shot-scraper CLI usage".into(),
            age: "18h",
            pinned: false,
            running: false,
            lines: shot_scraper_doc(),
        },
        Thread {
            id: 5,
            project: "shot-scraper",
            title: "Run this: uv run pytest -x".into(),
            age: "1d",
            pinned: false,
            running: false,
            lines: vec![system("pytest -x finished clean.")],
        },
    ]
}

fn system(body: &str) -> Line {
    Line {
        speaker: Speaker::System,
        body: body.into(),
    }
}

fn user(body: &str) -> Line {
    Line {
        speaker: Speaker::User,
        body: body.into(),
    }
}

fn shot_scraper_doc() -> Vec<Line> {
    vec![
        user("Document how to use the shot-scraper CLI tool"),
        system(
            "Started by reading the CLI entrypoint and the existing usage \
             notes. The command surface has six stable operations.",
        ),
        system(
            "Added focused examples for capture, crop, annotate, and export. \
             Each example uses the same sample directory.",
        ),
        system(
            "Updated `docs/cli.md` with exit codes and failure cases. The \
             reference now matches the parser help text.",
        ),
        system("Ran the focused command tests and checked every example."),
        system(
            "Done — the CLI guide is current.\n\
             Key outputs:\n\
             • Usage guide: `docs/cli.md`\n\
             • Command examples: `examples/commands/`\n\
             • Focused checks: `tests/cli.rs`",
        ),
    ]
}
