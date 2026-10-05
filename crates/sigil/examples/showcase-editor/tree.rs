use sigil::prelude::*;
use sigil::style::space;
use sigil::widgets::{ListRowSlots, RowTone};

use super::project::{FileId, Folder, GitStatus, PROJECT_NAME, TREE, TreeItem};
use super::state::{Activity, Workspace};

const ACTIVITIES: &[(Activity, &str, &str)] = &[
    (Activity::Explorer, "▤", "Explorer"),
    (Activity::Search, "◎", "Search"),
    (Activity::SourceControl, "◆", "Source control"),
    (Activity::Run, "▶", "Run and debug"),
];

pub(crate) fn activity_strip(ws: Workspace) -> NodeBuilder {
    ACTIVITIES.iter().fold(
        container().layout(
            LayoutStyle::flex_column()
                .fill_height()
                .shrink(0.0)
                .gap(space(1))
                .align_items(AlignItems::Center),
        ),
        |strip, (activity, glyph, name)| {
            let activity = *activity;
            strip.child(container().children_reactive(move || {
                let base = icon_button(*glyph, move || ws.activity.set(activity))
                    .accessible(*name)
                    .size(ControlSize::Default);
                [if ws.activity.get() == activity {
                    base.soft().tone(Tone::Primary)
                } else {
                    base.ghost()
                }]
            }))
        },
    )
}

pub(crate) fn side_bar(ws: Workspace) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().fill().min_size(0.0, 0.0))
        .bg(Token::SurfaceSubtle)
        .rounded(Radius::Md)
        .child(
            switch(ws.activity)
                .case(Activity::Explorer, move || explorer(ws))
                .case(Activity::SourceControl, move || source_control(ws))
                .case(Activity::Search, || {
                    empty_state("Search", "Type in the command palette to search files.")
                })
                .case(Activity::Run, || {
                    empty_state("Run and debug", "Use Run tests in the top bar.")
                }),
        )
}

fn header(title: &'static str) -> NodeBuilder {
    section_label(title)
}

fn explorer(ws: Workspace) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().fill_width().padding_y(space(1)))
        .child(header("EXPLORER"))
        .child(list_row_with(
            false,
            PROJECT_NAME.to_uppercase(),
            ListRowSlots {
                disclosure: Disclosure::Expanded,
                tone: RowTone::Muted,
                ..ListRowSlots::default()
            },
        ))
        .child(
            container()
                .layout(LayoutStyle::flex_column().fill_width())
                .children_reactive(move || {
                    let expanded = ws.expanded.get();
                    let active = ws.active();
                    let dirty = ws.dirty.get();
                    TREE.iter()
                        .copied()
                        .filter(|item| item.ancestors().iter().all(|f| expanded.contains(f)))
                        .map(|item| tree_row(ws, item, &expanded, active, &dirty))
                        .collect::<Vec<_>>()
                }),
        )
}

fn tree_row(
    ws: Workspace,
    item: TreeItem,
    expanded: &[Folder],
    active: Option<FileId>,
    dirty: &[FileId],
) -> NodeBuilder {
    let depth = item.depth().saturating_add(1);
    match item {
        TreeItem::Folder(folder) => {
            let open = expanded.contains(&folder);
            list_row_with(
                false,
                folder.name(),
                ListRowSlots {
                    depth,
                    disclosure: if open {
                        Disclosure::Expanded
                    } else {
                        Disclosure::Collapsed
                    },
                    ..ListRowSlots::default()
                },
            )
            .pointer_down(move |_| ws.toggle_folder(folder))
        }
        TreeItem::File(file) => {
            let spec = file.spec();
            list_row_with(
                active == Some(file),
                spec.name,
                ListRowSlots {
                    depth,
                    disclosure: Disclosure::Leaf,
                    marker: Some(spec.kind.badge().into()),
                    detail: dirty.contains(&file).then_some("●".into()),
                    status: spec.git.map(|git| git.letter().into()),
                    ..ListRowSlots::default()
                },
            )
            .pointer_down(move |_| ws.open_file(file))
        }
    }
}

fn source_control(ws: Workspace) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_column()
                .fill_width()
                .padding_y(space(1))
                .gap(space(1)),
        )
        .child(header("SOURCE CONTROL"))
        .child(
            container()
                .layout(LayoutStyle::flex_column().fill_width())
                .children_reactive(move || {
                    let staged = ws.staged.get();
                    FileId::ALL
                        .into_iter()
                        .filter_map(|file| file.spec().git.map(|git| (file, git)))
                        .map(|(file, git)| change_row(ws, file, git, staged.contains(&file)))
                        .collect::<Vec<_>>()
                }),
        )
}

fn change_row(ws: Workspace, file: FileId, git: GitStatus, staged: bool) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .padding_x(space(2))
                .gap(space(2))
                .align_items(AlignItems::Center),
        )
        .child(chip(git.letter(), git.tone()))
        .child(
            container()
                .layout(LayoutStyle::flex_row().grow(1.0).min_size(0.0, 0.0))
                .pointer_down(move |_| ws.open_file(file))
                .child(text(file.path()).ellipsize().font_size(super::TEXT_SM)),
        )
        .child(
            button(if staged { "Unstage" } else { "Stage" })
                .ghost()
                .size(ControlSize::Compact)
                .activate(move || ws.toggle_stage(file)),
        )
}
